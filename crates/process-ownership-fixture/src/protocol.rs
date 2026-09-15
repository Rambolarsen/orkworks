//! Authenticated control protocol shared by the process-ownership fixture roles.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

/// Maximum encoded size of one newline-delimited JSON message, including its newline.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

const RANDOM_VALUE_BYTES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

/// Identifies one owner/supervisor generation.
pub type GenerationId = u64;

/// Closed set of process-root ownership roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The sidecar-shaped process root.
    Sidecar,
    /// A PTY-shaped process root.
    Pty,
    /// An inference process root.
    Inference,
}

/// A supervisor-issued capability authorizing one process-root launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchTicket {
    /// Generation in which the launch is authorized.
    pub generation: GenerationId,
    /// Ownership role authorized by this ticket.
    pub role: Role,
    /// Fixture-owned executable identity authorized by this ticket.
    pub executable_identity: String,
    /// Nonce that binds the caller's launch request.
    pub request_nonce: String,
    /// Supervisor-generated one-use ticket identifier.
    pub ticket_id: String,
}

/// Native process identity retained by the live supervisor.
///
/// Equality and hashing intentionally ignore [`Self::diagnostic_pid`]. Only the
/// platform birth identity participates in ownership comparisons.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeIdentity {
    /// Platform-specific identity that distinguishes PID reuse.
    pub birth_identity: String,
    /// PID reported only for diagnostics and evidence output.
    pub diagnostic_pid: u32,
}

impl PartialEq for NativeIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.birth_identity == other.birth_identity
    }
}

impl Eq for NativeIdentity {}

impl Hash for NativeIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.birth_identity.hash(state);
    }
}

/// Receipt produced only after independent observation finds no owned survivor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteExitReceipt {
    /// Generation whose process set was observed complete.
    pub generation: GenerationId,
    /// Native identities included in the completed ownership census.
    pub owned_processes: Vec<NativeIdentity>,
    /// Supervisor observation time in Unix epoch milliseconds.
    pub observed_at_ms: u128,
}

/// Persisted rendezvous state for one supervisor generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "detail",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RendezvousState {
    /// The supervisor remains live; no exit receipt is available.
    Live,
    /// The live supervisor authenticated a complete-exit receipt.
    CompleteExit(CompleteExitReceipt),
    /// Ownership could not be established or completed.
    Unresolved(String),
}

/// Persisted locator for one live or completed supervisor generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RendezvousRecord {
    /// Generation served by the endpoint.
    pub generation: GenerationId,
    /// Supervisor-generated rendezvous nonce.
    pub nonce: String,
    /// Local IPC endpoint locator.
    pub endpoint: String,
    /// Last supervisor-authored rendezvous state.
    pub state: RendezvousState,
}

/// Values returned after preparing one supervisor generation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedGeneration {
    /// Persistable endpoint locator and state.
    pub record: RendezvousRecord,
    /// Supervisor-generated secret used by the owner to authenticate IPC commands.
    pub endpoint_token: String,
}

impl fmt::Debug for PreparedGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedGeneration")
            .field("record", &self.record)
            .field("endpoint_token", &"[REDACTED]")
            .finish()
    }
}

impl PreparedGeneration {
    /// Signs an authenticated request for this prepared generation.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::InvalidEndpointToken`] if the endpoint token is
    /// not the supervisor-generated 256-bit hexadecimal secret, or
    /// [`ProtocolError::InvalidMessage`] if the signing payload cannot serialize.
    pub fn authenticated_command(
        &self,
        request: AuthenticatedRequest,
    ) -> Result<FixtureCommand, ProtocolError> {
        let secret = decode_endpoint_token(&self.endpoint_token)?;
        FixtureCommand::authenticated(
            self.record.generation,
            self.record.nonce.clone(),
            &secret,
            request,
        )
    }
}

/// Commands that require generation, rendezvous, and authentication binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AuthenticatedRequest {
    /// Requests a one-use launch ticket.
    IssueLaunchTicket {
        /// Requested ownership role.
        role: Role,
        /// Fixture-owned executable identity selected by the supervisor layer.
        executable_identity: String,
        /// Nonce for this launch request.
        request_nonce: String,
    },
    /// Requests admission of a process root using a launch ticket.
    Spawn {
        /// Supervisor-issued launch ticket.
        ticket: LaunchTicket,
        /// Whether the root remains paused after ownership registration.
        paused: bool,
    },
    /// Supplies an untrusted diagnostic readiness report from a target.
    Ready {
        /// Role reported by the target.
        role: Role,
        /// Ticket identifier reported by the target.
        ticket_id: String,
        /// PID reported only for diagnostics.
        diagnostic_pid: u32,
    },
    /// Requests an independent supervisor/OS ownership observation.
    Observe,
    /// Reports orderly loss of the owner's control channel.
    OwnerLost,
    /// Requests bounded cleanup of the prepared generation.
    Cleanup,
    /// Requests independent status for the foreign-owner fixture.
    ForeignStatus,
    /// Requests live rendezvous state for a generation and nonce.
    RendezvousStatus {
        /// Generation being queried.
        target_generation: GenerationId,
        /// Rendezvous nonce being queried.
        target_nonce: String,
    },
    /// Requests adoption using a live-authenticated rendezvous record.
    Adopt {
        /// Record returned by the live rendezvous exchange, if available.
        record: Option<RendezvousRecord>,
    },
}

/// One newline-delimited command sent to the fixture supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FixtureCommand {
    /// Initializes one generation before authenticated IPC exists.
    Prepare {
        /// Generation to prepare.
        generation: GenerationId,
    },
    /// Carries one request authenticated by the prepared endpoint token.
    Authenticated {
        /// Generation against which the command must be checked.
        generation: GenerationId,
        /// Supervisor-generated rendezvous nonce.
        rendezvous_nonce: String,
        /// HMAC-SHA-256 tag over the generation, nonce, operation, and payload.
        authentication_tag: String,
        /// Closed authenticated operation and payload.
        request: AuthenticatedRequest,
    },
}

impl FixtureCommand {
    fn authenticated(
        generation: GenerationId,
        rendezvous_nonce: String,
        secret: &[u8; RANDOM_VALUE_BYTES],
        request: AuthenticatedRequest,
    ) -> Result<Self, ProtocolError> {
        let signing_bytes = command_signing_bytes(generation, &rendezvous_nonce, &request)?;
        let authentication_tag = hex::encode(hmac_sha256(secret, &signing_bytes));
        Ok(Self::Authenticated {
            generation,
            rendezvous_nonce,
            authentication_tag,
            request,
        })
    }
}

/// Independent observation of the generation's current owned process set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "detail",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnerObservation {
    /// The supervisor independently confirmed that no owned process remains.
    Empty,
    /// The supervisor independently observed owned native identities.
    Owned(Vec<NativeIdentity>),
    /// The owner or observer could not establish a complete process census.
    Unresolved(String),
}

/// One newline-delimited reply emitted by the fixture supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "reply",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FixtureReply {
    /// Returns a newly prepared generation and endpoint token.
    Prepared(PreparedGeneration),
    /// Returns a supervisor-issued launch ticket.
    LaunchTicket(LaunchTicket),
    /// Confirms an authenticated operation with no additional payload.
    Acknowledged,
    /// Returns one native process identity.
    Spawned(NativeIdentity),
    /// Returns an independent ownership observation.
    Observation {
        /// Confirmed, owned, or unresolved state.
        observation: OwnerObservation,
    },
    /// Returns live-authenticated rendezvous state.
    Rendezvous {
        /// State authenticated through the live endpoint.
        state: RendezvousState,
    },
    /// Reports a rejected command without exposing secret material.
    Rejected {
        /// Stable fixture diagnostic.
        reason: String,
    },
}

/// Protocol validation and framing failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A command or receipt refers to another generation.
    #[error("wrong generation: expected {expected}, received {actual}")]
    WrongGeneration {
        /// Prepared generation.
        expected: GenerationId,
        /// Generation supplied by the caller.
        actual: GenerationId,
    },
    /// A ticket does not match its supervisor-issued binding.
    #[error("launch ticket binding does not match")]
    TicketBindingMismatch,
    /// A ticket identifier was not issued by this supervisor generation.
    #[error("launch ticket was not issued by this generation")]
    UnknownTicket,
    /// A one-use ticket was presented more than once.
    #[error("launch ticket was already consumed")]
    TicketAlreadyConsumed,
    /// A command carried a stale or foreign rendezvous record.
    #[error("rendezvous record is stale or foreign")]
    StaleRendezvous,
    /// A command's authentication tag did not verify.
    #[error("command authentication failed")]
    AuthenticationFailed,
    /// A caller attempted to use prepare after authenticated IPC was established.
    #[error("prepare is valid only as the initial request")]
    PrepareMustBeInitial,
    /// Adoption cannot distinguish the owner state from an unresolved result.
    #[error("adoption remains unresolved: {0}")]
    AdoptionUnresolved(String),
    /// A complete-exit state lacked required independent-observation evidence.
    #[error("complete-exit receipt is incomplete: {0}")]
    IncompleteReceipt(String),
    /// An endpoint token was not a supervisor-generated 256-bit value.
    #[error("endpoint token is invalid")]
    InvalidEndpointToken,
    /// A caller supplied an empty or control-bearing protocol value.
    #[error("invalid {field}")]
    InvalidValue {
        /// Field rejected at the protocol boundary.
        field: &'static str,
    },
    /// OS randomness was unavailable while generating a capability.
    #[error("OS randomness unavailable: {0}")]
    RandomnessUnavailable(String),
    /// A frame exceeded [`MAX_MESSAGE_BYTES`].
    #[error("message is too large: {actual} bytes exceeds {max}")]
    MessageTooLarge {
        /// Encoded message length.
        actual: usize,
        /// Maximum permitted message length.
        max: usize,
    },
    /// A frame did not end with a newline.
    #[error("message is missing its newline delimiter")]
    MissingNewline,
    /// More than one newline-delimited message was supplied to a single decode.
    #[error("multiple messages were supplied")]
    MultipleMessages,
    /// JSON syntax, shape, or protocol values were invalid.
    #[error("invalid protocol message: {0}")]
    InvalidMessage(String),
}

/// In-memory supervisor authority for one prepared generation.
///
/// This value retains the raw endpoint secret and ticket-consumption set. It is
/// deliberately neither serializable nor cloneable.
pub struct SupervisorProtocol {
    generation: GenerationId,
    rendezvous_nonce: String,
    endpoint: String,
    endpoint_secret: [u8; RANDOM_VALUE_BYTES],
    issued_tickets: HashMap<String, LaunchTicket>,
    consumed_tickets: HashSet<String>,
}

impl fmt::Debug for SupervisorProtocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupervisorProtocol")
            .field("generation", &self.generation)
            .field("rendezvous_nonce", &self.rendezvous_nonce)
            .field("endpoint", &self.endpoint)
            .field("endpoint_secret", &"[REDACTED]")
            .field("issued_ticket_count", &self.issued_tickets.len())
            .field("consumed_ticket_count", &self.consumed_tickets.len())
            .finish()
    }
}

impl SupervisorProtocol {
    /// Prepares a generation and generates its rendezvous nonce and endpoint token.
    ///
    /// No process target is launched by this operation.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::InvalidValue`] for an empty or control-bearing
    /// endpoint, or [`ProtocolError::RandomnessUnavailable`] when the operating
    /// system cannot provide capability randomness.
    pub fn prepare(
        generation: GenerationId,
        endpoint: impl Into<String>,
    ) -> Result<(Self, PreparedGeneration), ProtocolError> {
        let endpoint = endpoint.into();
        validate_text("endpoint", &endpoint)?;
        let rendezvous_nonce = random_hex()?;
        let endpoint_secret = random_bytes()?;
        let endpoint_token = hex::encode(endpoint_secret);
        let record = RendezvousRecord {
            generation,
            nonce: rendezvous_nonce.clone(),
            endpoint: endpoint.clone(),
            state: RendezvousState::Live,
        };
        let prepared = PreparedGeneration {
            record,
            endpoint_token,
        };
        let protocol = Self {
            generation,
            rendezvous_nonce,
            endpoint,
            endpoint_secret,
            issued_tickets: HashMap::new(),
            consumed_tickets: HashSet::new(),
        };
        Ok((protocol, prepared))
    }

    /// Issues a one-use ticket bound to the prepared generation and launch request.
    ///
    /// # Errors
    ///
    /// Returns [`ProtocolError::InvalidValue`] for empty or control-bearing
    /// binding values, or [`ProtocolError::RandomnessUnavailable`] if a ticket ID
    /// cannot be generated.
    pub fn issue_launch_ticket(
        &mut self,
        role: Role,
        executable_identity: impl Into<String>,
        request_nonce: impl Into<String>,
    ) -> Result<LaunchTicket, ProtocolError> {
        let executable_identity = executable_identity.into();
        let request_nonce = request_nonce.into();
        validate_text("executable identity", &executable_identity)?;
        validate_text("request nonce", &request_nonce)?;
        let ticket = LaunchTicket {
            generation: self.generation,
            role,
            executable_identity,
            request_nonce,
            ticket_id: random_hex()?,
        };
        self.issued_tickets
            .insert(ticket.ticket_id.clone(), ticket.clone());
        Ok(ticket)
    }

    /// Consumes a ticket only when every generation and launch binding matches.
    ///
    /// A rejected forged ticket does not consume the valid supervisor-issued
    /// ticket with the same identifier.
    ///
    /// # Errors
    ///
    /// Returns a generation, binding, unknown-ticket, or replay error when the
    /// capability is not valid for this exact launch request.
    pub fn accept_ticket(
        &mut self,
        ticket: LaunchTicket,
        expected_role: Role,
        expected_executable_identity: &str,
        expected_request_nonce: &str,
    ) -> Result<(), ProtocolError> {
        self.check_generation(ticket.generation)?;
        if self.consumed_tickets.contains(&ticket.ticket_id) {
            return Err(ProtocolError::TicketAlreadyConsumed);
        }
        let Some(issued) = self.issued_tickets.get(&ticket.ticket_id) else {
            return Err(ProtocolError::UnknownTicket);
        };
        if issued != &ticket
            || ticket.role != expected_role
            || ticket.executable_identity != expected_executable_identity
            || ticket.request_nonce != expected_request_nonce
        {
            return Err(ProtocolError::TicketBindingMismatch);
        }
        self.consumed_tickets.insert(ticket.ticket_id);
        Ok(())
    }

    /// Verifies one authenticated command against the live supervisor secret.
    ///
    /// # Errors
    ///
    /// Returns a generation, rendezvous, framing, or authentication error if the
    /// command was not created for this exact live endpoint. A second prepare
    /// request is rejected because preparation is the sole unauthenticated step.
    pub fn verify_command<'a>(
        &self,
        command: &'a FixtureCommand,
    ) -> Result<&'a AuthenticatedRequest, ProtocolError> {
        let FixtureCommand::Authenticated {
            generation,
            rendezvous_nonce,
            authentication_tag,
            request,
        } = command
        else {
            return Err(ProtocolError::PrepareMustBeInitial);
        };
        self.check_generation(*generation)?;
        if rendezvous_nonce != &self.rendezvous_nonce {
            return Err(ProtocolError::StaleRendezvous);
        }
        let signing_bytes = command_signing_bytes(*generation, rendezvous_nonce, request)?;
        let supplied_tag =
            hex::decode(authentication_tag).map_err(|_| ProtocolError::AuthenticationFailed)?;
        let mut mac = HmacSha256::new_from_slice(&self.endpoint_secret)
            .map_err(|_| ProtocolError::InvalidEndpointToken)?;
        mac.update(&signing_bytes);
        mac.verify_slice(&supplied_tag)
            .map_err(|_| ProtocolError::AuthenticationFailed)?;
        Ok(request)
    }

    /// Validates adoption against this live-authenticated rendezvous generation.
    ///
    /// # Errors
    ///
    /// Missing, live, unresolved, stale, or incomplete states are rejected. A
    /// complete-exit receipt is accepted only for this exact generation and
    /// rendezvous record.
    pub fn adopt(
        &self,
        record: Option<&RendezvousRecord>,
    ) -> Result<CompleteExitReceipt, ProtocolError> {
        let Some(record) = record else {
            return Err(ProtocolError::AdoptionUnresolved(
                "rendezvous record is missing".to_owned(),
            ));
        };
        self.check_generation(record.generation)?;
        if record.nonce != self.rendezvous_nonce || record.endpoint != self.endpoint {
            return Err(ProtocolError::StaleRendezvous);
        }
        let receipt = match &record.state {
            RendezvousState::Live => {
                return Err(ProtocolError::AdoptionUnresolved(
                    "owner remains live".to_owned(),
                ));
            }
            RendezvousState::Unresolved(reason) => {
                return Err(ProtocolError::AdoptionUnresolved(reason.clone()));
            }
            RendezvousState::CompleteExit(receipt) => receipt,
        };
        self.check_generation(receipt.generation)?;
        validate_complete_exit_receipt(receipt)?;
        Ok(receipt.clone())
    }

    fn check_generation(&self, actual: GenerationId) -> Result<(), ProtocolError> {
        if actual != self.generation {
            return Err(ProtocolError::WrongGeneration {
                expected: self.generation,
                actual,
            });
        }
        Ok(())
    }
}

/// Encodes one fixture command as bounded newline-delimited JSON.
///
/// # Errors
///
/// Returns [`ProtocolError::InvalidMessage`] when serialization fails or
/// [`ProtocolError::MessageTooLarge`] when the encoded line exceeds 64 KiB.
pub fn encode_command_line(command: &FixtureCommand) -> Result<Vec<u8>, ProtocolError> {
    encode_line(command)
}

/// Decodes one bounded newline-delimited JSON fixture command.
///
/// # Errors
///
/// Rejects missing/multiple newlines, messages over 64 KiB, malformed JSON,
/// unknown operations, unknown fields, and invalid enum variants.
pub fn decode_command_line(line: &[u8]) -> Result<FixtureCommand, ProtocolError> {
    decode_line(line)
}

/// Encodes one fixture reply as bounded newline-delimited JSON.
///
/// # Errors
///
/// Returns [`ProtocolError::InvalidMessage`] when serialization fails or
/// [`ProtocolError::MessageTooLarge`] when the encoded line exceeds 64 KiB.
pub fn encode_reply_line(reply: &FixtureReply) -> Result<Vec<u8>, ProtocolError> {
    encode_line(reply)
}

/// Decodes one bounded newline-delimited JSON fixture reply.
///
/// # Errors
///
/// Rejects missing/multiple newlines, messages over 64 KiB, malformed JSON,
/// unknown replies, unknown fields, and invalid enum variants.
pub fn decode_reply_line(line: &[u8]) -> Result<FixtureReply, ProtocolError> {
    decode_line(line)
}

fn validate_complete_exit_receipt(receipt: &CompleteExitReceipt) -> Result<(), ProtocolError> {
    if receipt.observed_at_ms == 0 {
        return Err(ProtocolError::IncompleteReceipt(
            "observation timestamp is missing".to_owned(),
        ));
    }
    if receipt
        .owned_processes
        .iter()
        .any(|identity| !is_valid_text(&identity.birth_identity))
    {
        return Err(ProtocolError::IncompleteReceipt(
            "native birth identity is missing".to_owned(),
        ));
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), ProtocolError> {
    if !is_valid_text(value) {
        return Err(ProtocolError::InvalidValue { field });
    }
    Ok(())
}

fn is_valid_text(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn random_hex() -> Result<String, ProtocolError> {
    random_bytes().map(hex::encode)
}

fn random_bytes() -> Result<[u8; RANDOM_VALUE_BYTES], ProtocolError> {
    let mut bytes = [0_u8; RANDOM_VALUE_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| ProtocolError::RandomnessUnavailable(error.to_string()))?;
    Ok(bytes)
}

fn decode_endpoint_token(token: &str) -> Result<[u8; RANDOM_VALUE_BYTES], ProtocolError> {
    let decoded = hex::decode(token).map_err(|_| ProtocolError::InvalidEndpointToken)?;
    decoded
        .try_into()
        .map_err(|_| ProtocolError::InvalidEndpointToken)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CommandSigningPayload<'a> {
    generation: GenerationId,
    rendezvous_nonce: &'a str,
    request: &'a AuthenticatedRequest,
}

fn command_signing_bytes(
    generation: GenerationId,
    rendezvous_nonce: &str,
    request: &AuthenticatedRequest,
) -> Result<Vec<u8>, ProtocolError> {
    serde_json::to_vec(&CommandSigningPayload {
        generation,
        rendezvous_nonce,
        request,
    })
    .map_err(|error| ProtocolError::InvalidMessage(error.to_string()))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA-256 accepts keys of every size");
    mac.update(message);
    mac.finalize().into_bytes().into()
}

fn encode_line<T: Serialize>(message: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| ProtocolError::InvalidMessage(error.to_string()))?;
    let encoded_len = encoded
        .len()
        .checked_add(1)
        .ok_or(ProtocolError::MessageTooLarge {
            actual: usize::MAX,
            max: MAX_MESSAGE_BYTES,
        })?;
    if encoded_len > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge {
            actual: encoded_len,
            max: MAX_MESSAGE_BYTES,
        });
    }
    encoded.push(b'\n');
    Ok(encoded)
}

fn decode_line<T: DeserializeOwned>(line: &[u8]) -> Result<T, ProtocolError> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge {
            actual: line.len(),
            max: MAX_MESSAGE_BYTES,
        });
    }
    let Some(body) = line.strip_suffix(b"\n") else {
        return Err(ProtocolError::MissingNewline);
    };
    if body.contains(&b'\n') {
        return Err(ProtocolError::MultipleMessages);
    }
    let body = body.strip_suffix(b"\r").unwrap_or(body);
    serde_json::from_slice(body).map_err(|error| ProtocolError::InvalidMessage(error.to_string()))
}
