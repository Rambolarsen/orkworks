#[path = "../src/protocol.rs"]
mod protocol;

use protocol::{
    decode_command_line, decode_reply_line, encode_command_line, encode_reply_line,
    AuthenticatedRendezvousReply, AuthenticatedRequest, CleanupResult, CompleteExitReceipt,
    FixtureCommand, FixtureReply, NativeIdentity, OwnerObservation, ProtocolError, RendezvousState,
    Role, SupervisorProtocol, MAX_MESSAGE_BYTES,
};

const GENERATION: u64 = 7;

fn prepared_protocol() -> (SupervisorProtocol, protocol::PreparedGeneration) {
    SupervisorProtocol::prepare(GENERATION).expect("fixture generation should prepare")
}

fn valid_receipt() -> CompleteExitReceipt {
    CompleteExitReceipt {
        generation: GENERATION,
        owned_processes: vec![NativeIdentity {
            birth_identity: "darwin:123:456789".to_owned(),
            diagnostic_pid: 123,
        }],
        observed_at_ms: 1_789_472_000_000,
    }
}

#[test]
fn prepare_generates_fresh_rendezvous_nonce_and_endpoint_token() {
    let (_, first) = prepared_protocol();
    let (_, second) = prepared_protocol();

    assert_eq!(first.record.generation, GENERATION);
    assert!(first.record.endpoint.starts_with("fixture-rendezvous:"));
    assert!(matches!(first.record.state, RendezvousState::Live));
    assert_eq!(first.record.nonce.len(), 64);
    assert_eq!(first.endpoint_token.len(), 64);
    assert_ne!(first.record.endpoint, second.record.endpoint);
    assert_ne!(first.record.nonce, second.record.nonce);
    assert_ne!(first.endpoint_token, second.endpoint_token);
}

#[test]
fn role_rejects_target_behavior_names() {
    let result = serde_json::from_str::<Role>(r#""daemonized""#);

    assert!(result.is_err());
}

#[test]
fn ticket_rejects_wrong_generation_without_consuming_original() {
    let (mut supervisor, _) = prepared_protocol();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-a")
        .expect("ticket should be issued");
    let mut forged = ticket.clone();
    forged.generation = GENERATION + 1;

    let forged_result =
        supervisor.accept_ticket(forged, Role::Inference, "fixture-inference", "req-a");
    let original_result =
        supervisor.accept_ticket(ticket, Role::Inference, "fixture-inference", "req-a");

    assert!(matches!(
        forged_result,
        Err(ProtocolError::WrongGeneration { .. })
    ));
    assert!(original_result.is_ok());
}

#[test]
fn ticket_rejects_wrong_role_without_consuming_original() {
    let (mut supervisor, _) = prepared_protocol();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-a")
        .expect("ticket should be issued");

    let forged_result =
        supervisor.accept_ticket(ticket.clone(), Role::Pty, "fixture-inference", "req-a");
    let original_result =
        supervisor.accept_ticket(ticket, Role::Inference, "fixture-inference", "req-a");

    assert!(matches!(
        forged_result,
        Err(ProtocolError::TicketBindingMismatch)
    ));
    assert!(original_result.is_ok());
}

#[test]
fn ticket_rejects_wrong_executable_identity_without_consuming_original() {
    let (mut supervisor, _) = prepared_protocol();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-a")
        .expect("ticket should be issued");

    let forged_result = supervisor.accept_ticket(
        ticket.clone(),
        Role::Inference,
        "foreign-inference",
        "req-a",
    );
    let original_result =
        supervisor.accept_ticket(ticket, Role::Inference, "fixture-inference", "req-a");

    assert!(matches!(
        forged_result,
        Err(ProtocolError::TicketBindingMismatch)
    ));
    assert!(original_result.is_ok());
}

#[test]
fn ticket_rejects_wrong_request_nonce_without_consuming_original() {
    let (mut supervisor, _) = prepared_protocol();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-a")
        .expect("ticket should be issued");

    let forged_result = supervisor.accept_ticket(
        ticket.clone(),
        Role::Inference,
        "fixture-inference",
        "req-b",
    );
    let original_result =
        supervisor.accept_ticket(ticket, Role::Inference, "fixture-inference", "req-a");

    assert!(matches!(
        forged_result,
        Err(ProtocolError::TicketBindingMismatch)
    ));
    assert!(original_result.is_ok());
}

#[test]
fn ticket_issuance_rejects_foreign_executable_identity() {
    let (mut supervisor, _) = prepared_protocol();

    let result = supervisor.issue_launch_ticket(Role::Inference, "foreign-inference", "req-a");

    assert!(matches!(
        result,
        Err(ProtocolError::ExecutableIdentityRejected { .. })
    ));
}

#[test]
fn ticket_issuance_rejects_role_incompatible_executable_identity() {
    let (mut supervisor, _) = prepared_protocol();

    let result = supervisor.issue_launch_ticket(Role::Pty, "fixture-inference", "req-a");

    assert!(matches!(
        result,
        Err(ProtocolError::ExecutableIdentityRejected { .. })
    ));
}

#[test]
fn ticket_id_is_rejected_after_one_use() {
    let (mut supervisor, _) = prepared_protocol();
    let ticket = supervisor
        .issue_launch_ticket(Role::Inference, "fixture-inference", "req-a")
        .expect("ticket should be issued");

    let first = supervisor.accept_ticket(
        ticket.clone(),
        Role::Inference,
        "fixture-inference",
        "req-a",
    );
    let replay = supervisor.accept_ticket(ticket, Role::Inference, "fixture-inference", "req-a");

    assert!(first.is_ok());
    assert!(matches!(replay, Err(ProtocolError::TicketAlreadyConsumed)));
}

#[test]
fn authenticated_command_is_bound_to_generation_nonce_operation_and_payload() {
    let (supervisor, prepared) = prepared_protocol();
    let request = AuthenticatedRequest::IssueLaunchTicket {
        role: Role::Pty,
        executable_identity: "fixture-pty".to_owned(),
        request_nonce: "req-pty".to_owned(),
    };
    let command = prepared
        .authenticated_command(request.clone())
        .expect("command should authenticate");

    let verified = supervisor.verify_command(&command);

    assert_eq!(verified, Ok(&request));
}

#[test]
fn authenticated_command_rejects_payload_tampering() {
    let (supervisor, prepared) = prepared_protocol();
    let mut command = prepared
        .authenticated_command(AuthenticatedRequest::IssueLaunchTicket {
            role: Role::Pty,
            executable_identity: "fixture-pty".to_owned(),
            request_nonce: "req-pty".to_owned(),
        })
        .expect("command should authenticate");
    let FixtureCommand::Authenticated { request, .. } = &mut command else {
        panic!("expected authenticated command");
    };
    *request = AuthenticatedRequest::Observe;

    let result = supervisor.verify_command(&command);

    assert!(matches!(result, Err(ProtocolError::AuthenticationFailed)));
}

#[test]
fn authenticated_command_rejects_modified_authentication_tag() {
    let (supervisor, prepared) = prepared_protocol();
    let mut command = prepared
        .authenticated_command(AuthenticatedRequest::Observe)
        .expect("command should authenticate");
    let FixtureCommand::Authenticated {
        authentication_tag, ..
    } = &mut command
    else {
        panic!("expected authenticated command");
    };
    let replacement = if authentication_tag.starts_with('0') {
        "1"
    } else {
        "0"
    };
    authentication_tag.replace_range(..1, replacement);

    let result = supervisor.verify_command(&command);

    assert!(matches!(result, Err(ProtocolError::AuthenticationFailed)));
}

#[test]
fn authenticated_command_rejects_stale_generation_and_nonce() {
    let (supervisor, prepared) = prepared_protocol();
    let command = prepared
        .authenticated_command(AuthenticatedRequest::Observe)
        .expect("command should authenticate");
    let mut stale_generation = command.clone();
    let FixtureCommand::Authenticated { generation, .. } = &mut stale_generation else {
        panic!("expected authenticated command");
    };
    *generation += 1;
    let mut stale_nonce = command;
    let FixtureCommand::Authenticated {
        rendezvous_nonce, ..
    } = &mut stale_nonce
    else {
        panic!("expected authenticated command");
    };
    *rendezvous_nonce = "stale-nonce".to_owned();

    assert!(matches!(
        supervisor.verify_command(&stale_generation),
        Err(ProtocolError::WrongGeneration { .. })
    ));
    assert!(matches!(
        supervisor.verify_command(&stale_nonce),
        Err(ProtocolError::StaleRendezvous)
    ));
}

#[test]
fn only_prepare_is_accepted_without_authentication_fields() {
    let prepare = decode_command_line(
        br#"{"operation":"prepare","payload":{"generation":7}}
"#,
    );
    let unauthenticated_observe = decode_command_line(
        br#"{"operation":"observe","payload":{"generation":7}}
"#,
    );

    assert_eq!(
        prepare,
        Ok(FixtureCommand::Prepare {
            generation: GENERATION
        })
    );
    assert!(matches!(
        unauthenticated_observe,
        Err(ProtocolError::InvalidMessage(_))
    ));
}

#[test]
fn decoder_rejects_unknown_operation_and_unknown_fields() {
    let unknown_operation = decode_command_line(
        br#"{"operation":"explode","payload":{}}
"#,
    );
    let unknown_field = decode_command_line(
        br#"{"operation":"prepare","payload":{"generation":7,"unexpected":true}}
"#,
    );

    assert!(matches!(
        unknown_operation,
        Err(ProtocolError::InvalidMessage(_))
    ));
    assert!(matches!(
        unknown_field,
        Err(ProtocolError::InvalidMessage(_))
    ));
}

#[test]
fn newline_delimited_decoder_requires_exactly_one_complete_message() {
    let command = FixtureCommand::Prepare {
        generation: GENERATION,
    };
    let encoded = encode_command_line(&command).expect("command should encode");
    let without_newline = &encoded[..encoded.len() - 1];
    let mut two_messages = encoded.clone();
    two_messages.extend_from_slice(&encoded);

    assert!(matches!(
        decode_command_line(without_newline),
        Err(ProtocolError::MissingNewline)
    ));
    assert!(matches!(
        decode_command_line(&two_messages),
        Err(ProtocolError::MultipleMessages)
    ));
}

#[test]
fn decoder_rejects_messages_larger_than_64_kib() {
    let mut oversized = vec![b' '; MAX_MESSAGE_BYTES + 1];
    *oversized.last_mut().expect("oversized input is nonempty") = b'\n';

    let result = decode_command_line(&oversized);

    assert!(matches!(result, Err(ProtocolError::MessageTooLarge { .. })));
}

#[test]
fn framing_accepts_exactly_64_kib_and_rejects_oversized_encoding() {
    const PREFIX: &[u8] = br#"{"reply":"rejected","payload":{"reason":""#;
    const SUFFIX: &[u8] = b"\"}}\n";
    let reason_len = MAX_MESSAGE_BYTES - PREFIX.len() - SUFFIX.len();
    let reason = "x".repeat(reason_len);
    let reply = FixtureReply::Rejected {
        reason: reason.clone(),
    };
    let mut exact_line = Vec::with_capacity(MAX_MESSAGE_BYTES);
    exact_line.extend_from_slice(PREFIX);
    exact_line.extend_from_slice(reason.as_bytes());
    exact_line.extend_from_slice(SUFFIX);

    let decoded = decode_reply_line(&exact_line);
    let encoded = encode_reply_line(&reply);
    let oversized = encode_reply_line(&FixtureReply::Rejected {
        reason: format!("{reason}x"),
    });

    assert_eq!(exact_line.len(), MAX_MESSAGE_BYTES);
    assert_eq!(decoded, Ok(reply));
    assert_eq!(
        encoded.expect("exact-bound reply should encode").len(),
        MAX_MESSAGE_BYTES
    );
    assert!(matches!(
        oversized,
        Err(ProtocolError::MessageTooLarge { .. })
    ));
}

#[test]
fn command_and_reply_round_trip_as_newline_delimited_json() {
    let command = FixtureCommand::Prepare {
        generation: GENERATION,
    };
    let reply = FixtureReply::Observation {
        observation: OwnerObservation::Unresolved("owner unavailable".to_owned()),
    };

    let command_line = encode_command_line(&command).expect("command should encode");
    let reply_line = encode_reply_line(&reply).expect("reply should encode");

    assert_eq!(command_line.last(), Some(&b'\n'));
    assert_eq!(reply_line.last(), Some(&b'\n'));
    assert_eq!(decode_command_line(&command_line), Ok(command));
    assert_eq!(decode_reply_line(&reply_line), Ok(reply));
}

#[test]
fn adoption_rejects_missing_live_and_unresolved_authenticated_responses() {
    let (mut supervisor, prepared) = prepared_protocol();
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");
    let live = supervisor
        .answer_adoption(&request)
        .expect("live state should be authenticated");
    assert!(matches!(
        prepared.verify_adoption_response(&request, None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
    assert!(matches!(
        prepared.verify_adoption_response(&request, Some(&live)),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));

    let (mut supervisor, prepared) = prepared_protocol();
    supervisor
        .record_unresolved("observer unavailable")
        .expect("supervisor should retain unresolved state");
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");
    let unresolved = supervisor
        .answer_adoption(&request)
        .expect("unresolved state should be authenticated");
    assert!(matches!(
        prepared.verify_adoption_response(&request, Some(&unresolved)),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
}

#[test]
fn adoption_rejects_stale_generation_nonce_and_replayed_challenge() {
    let (mut supervisor, prepared) = prepared_protocol();
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");
    let mut stale_generation = request.clone();
    stale_generation.generation += 1;
    let mut stale_nonce = request.clone();
    stale_nonce.rendezvous_nonce = "stale-nonce".to_owned();

    assert!(matches!(
        supervisor.answer_adoption(&stale_generation),
        Err(ProtocolError::WrongGeneration { .. })
    ));
    assert!(matches!(
        supervisor.answer_adoption(&stale_nonce),
        Err(ProtocolError::StaleRendezvous)
    ));
    assert!(supervisor.answer_adoption(&request).is_ok());
    assert!(matches!(
        supervisor.answer_adoption(&request),
        Err(ProtocolError::ChallengeAlreadyUsed)
    ));
}

#[test]
fn adoption_rejects_incomplete_and_malformed_receipts() {
    let (mut supervisor, _) = prepared_protocol();
    let mut receipt = valid_receipt();
    receipt.observed_at_ms = 0;
    let malformed = decode_reply_line(
        br#"{"reply":"rendezvous","payload":{"response":{"generation":7,"rendezvous_nonce":"nonce","challenge":"challenge","state":{"state":"complete_exit","detail":{"generation":7,"owned_processes":[]}},"authentication_tag":"tag"}}}
"#,
    );

    assert!(matches!(
        supervisor.record_complete_exit(receipt),
        Err(ProtocolError::IncompleteReceipt(_))
    ));
    assert!(matches!(malformed, Err(ProtocolError::InvalidMessage(_))));
}

#[test]
fn adoption_rejects_semantically_incomplete_native_identity() {
    let (mut supervisor, _) = prepared_protocol();
    let mut receipt = valid_receipt();
    receipt.owned_processes[0].birth_identity.clear();

    let result = supervisor.record_complete_exit(receipt);

    assert!(matches!(result, Err(ProtocolError::IncompleteReceipt(_))));
}

#[test]
fn reply_framing_rejects_semantically_incomplete_native_identity() {
    let malformed = decode_reply_line(
        br#"{"reply":"spawned","payload":{"birth_identity":"","diagnostic_pid":123}}
"#,
    );

    assert!(matches!(malformed, Err(ProtocolError::InvalidValue { .. })));
}

#[test]
fn mutated_persisted_record_cannot_fabricate_complete_exit() {
    let (mut supervisor, mut prepared) = prepared_protocol();
    prepared.record.state = RendezvousState::CompleteExit(valid_receipt());
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");

    let response = supervisor
        .answer_adoption(&request)
        .expect("authoritative live state should be authenticated");
    let adopted = prepared.verify_adoption_response(&request, Some(&response));

    assert!(matches!(response.state, RendezvousState::Live));
    assert!(matches!(adopted, Err(ProtocolError::AdoptionUnresolved(_))));
}

#[test]
fn adoption_rejects_unsigned_and_spoofed_rendezvous_replies() {
    let (mut supervisor, prepared) = prepared_protocol();
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");
    let signed_live = supervisor
        .answer_adoption(&request)
        .expect("live response should be authenticated");
    let unsigned = AuthenticatedRendezvousReply {
        generation: request.generation,
        rendezvous_nonce: request.rendezvous_nonce.clone(),
        challenge: request.challenge.clone(),
        state: RendezvousState::CompleteExit(valid_receipt()),
        authentication_tag: String::new(),
    };
    let mut spoofed = signed_live;
    spoofed.state = RendezvousState::CompleteExit(valid_receipt());

    assert!(matches!(
        prepared.verify_adoption_response(&request, Some(&unsigned)),
        Err(ProtocolError::AuthenticationFailed)
    ));
    assert!(matches!(
        prepared.verify_adoption_response(&request, Some(&spoofed)),
        Err(ProtocolError::AuthenticationFailed)
    ));
}

#[test]
fn adoption_accepts_supervisor_authenticated_authoritative_complete_exit() {
    let (mut supervisor, prepared) = prepared_protocol();
    let receipt = valid_receipt();
    supervisor
        .record_complete_exit(receipt.clone())
        .expect("complete receipt should become authoritative");
    let request = prepared
        .adoption_request()
        .expect("challenge should be generated");
    let response = supervisor
        .answer_adoption(&request)
        .expect("complete state should be authenticated");

    let adopted = prepared.verify_adoption_response(&request, Some(&response));

    assert_eq!(adopted, Ok(receipt));
}

#[test]
fn cleanup_reply_distinguishes_receipt_acknowledgement_from_unresolved_survivors() {
    let acknowledged = FixtureReply::Cleanup {
        result: CleanupResult::Acknowledged(valid_receipt()),
    };
    let unresolved = FixtureReply::Cleanup {
        result: CleanupResult::Unresolved {
            survivors: vec![NativeIdentity {
                birth_identity: "darwin:456:789012".to_owned(),
                diagnostic_pid: 456,
            }],
            reason: "termination deadline elapsed".to_owned(),
        },
    };

    for reply in [acknowledged, unresolved] {
        let line = encode_reply_line(&reply).expect("cleanup reply should encode");
        assert_eq!(decode_reply_line(&line), Ok(reply));
    }

    let mut incomplete = valid_receipt();
    incomplete.observed_at_ms = 0;
    assert!(matches!(
        encode_reply_line(&FixtureReply::Cleanup {
            result: CleanupResult::Acknowledged(incomplete),
        }),
        Err(ProtocolError::IncompleteReceipt(_))
    ));
}

#[test]
fn unresolved_owner_is_distinct_from_confirmed_empty_owner() {
    assert_ne!(
        OwnerObservation::Unresolved("owner unavailable".to_owned()),
        OwnerObservation::Empty
    );
}

#[test]
fn native_identity_equality_uses_birth_identity_not_diagnostic_pid() {
    let original = NativeIdentity {
        birth_identity: "darwin:123:456789".to_owned(),
        diagnostic_pid: 123,
    };
    let same_birth_different_pid = NativeIdentity {
        birth_identity: "darwin:123:456789".to_owned(),
        diagnostic_pid: 999,
    };
    let reused_pid = NativeIdentity {
        birth_identity: "darwin:123:999999".to_owned(),
        diagnostic_pid: 123,
    };

    assert_eq!(original, same_birth_different_pid);
    assert_ne!(original, reused_pid);
}
