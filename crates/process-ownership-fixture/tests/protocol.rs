#[path = "../src/protocol.rs"]
mod protocol;

use protocol::{
    decode_command_line, decode_reply_line, encode_command_line, encode_reply_line,
    AuthenticatedRequest, CompleteExitReceipt, FixtureCommand, FixtureReply, NativeIdentity,
    OwnerObservation, ProtocolError, RendezvousState, Role, SupervisorProtocol, MAX_MESSAGE_BYTES,
};

const GENERATION: u64 = 7;
const ENDPOINT: &str = "fixture://supervisor-7";

fn prepared_protocol() -> (SupervisorProtocol, protocol::PreparedGeneration) {
    SupervisorProtocol::prepare(GENERATION, ENDPOINT).expect("fixture generation should prepare")
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
    assert_eq!(first.record.endpoint, ENDPOINT);
    assert!(matches!(first.record.state, RendezvousState::Live));
    assert_eq!(first.record.nonce.len(), 64);
    assert_eq!(first.endpoint_token.len(), 64);
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
fn adoption_rejects_missing_live_and_unresolved_owner_states() {
    let (supervisor, prepared) = prepared_protocol();
    let live = prepared.record;
    let mut unresolved = live.clone();
    unresolved.state = RendezvousState::Unresolved("observer unavailable".to_owned());

    assert!(matches!(
        supervisor.adopt(None),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
    assert!(matches!(
        supervisor.adopt(Some(&live)),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
    assert!(matches!(
        supervisor.adopt(Some(&unresolved)),
        Err(ProtocolError::AdoptionUnresolved(_))
    ));
}

#[test]
fn adoption_rejects_stale_generation_and_nonce() {
    let (supervisor, prepared) = prepared_protocol();
    let mut stale_generation = prepared.record.clone();
    stale_generation.generation += 1;
    stale_generation.state = RendezvousState::CompleteExit(valid_receipt());
    let mut stale_nonce = prepared.record;
    stale_nonce.nonce = "stale-nonce".to_owned();
    stale_nonce.state = RendezvousState::CompleteExit(valid_receipt());

    assert!(matches!(
        supervisor.adopt(Some(&stale_generation)),
        Err(ProtocolError::WrongGeneration { .. })
    ));
    assert!(matches!(
        supervisor.adopt(Some(&stale_nonce)),
        Err(ProtocolError::StaleRendezvous)
    ));
}

#[test]
fn adoption_rejects_incomplete_and_malformed_receipts() {
    let (supervisor, prepared) = prepared_protocol();
    let mut incomplete = prepared.record;
    let mut receipt = valid_receipt();
    receipt.observed_at_ms = 0;
    incomplete.state = RendezvousState::CompleteExit(receipt);
    let malformed = decode_reply_line(
        br#"{"reply":"rendezvous","payload":{"state":{"state":"complete_exit","receipt":{"generation":7,"owned_processes":[]}}}}
"#,
    );

    assert!(matches!(
        supervisor.adopt(Some(&incomplete)),
        Err(ProtocolError::IncompleteReceipt(_))
    ));
    assert!(matches!(malformed, Err(ProtocolError::InvalidMessage(_))));
}

#[test]
fn adoption_accepts_complete_exit_for_exact_generation_and_rendezvous() {
    let (supervisor, prepared) = prepared_protocol();
    let mut record = prepared.record;
    let receipt = valid_receipt();
    record.state = RendezvousState::CompleteExit(receipt.clone());

    let adopted = supervisor.adopt(Some(&record));

    assert_eq!(adopted, Ok(receipt));
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
