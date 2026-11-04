use super::*;
use std::sync::mpsc;
use std::time::Duration;

// Explicitly synthetic immutable context. No constructor is compiled for callers.
fn scope(o: &Authority, parent: Option<&Scope>) -> Scope {
    let node = Arc::new(ScopeState {
        owner: Arc::downgrade(&o.inner),
        closed: AtomicBool::new(false),
    });
    match parent {
        None => Scope {
            workspace: node.clone(),
            node,
            parents: [None, None],
        },
        Some(p) => {
            assert!(
                p.parents[1].is_none(),
                "fixed workspace/namespace/run ancestry"
            );
            Scope {
                node,
                workspace: p.workspace.clone(),
                parents: [Some(p.node.clone()), p.parents[0].clone()],
            }
        }
    }
}
fn binding(s: &Scope) -> Binding {
    Binding {
        data: Arc::new(BindingData {
            scope: s.clone(),
            phase: AtomicU8::new(0),
            source: Arc::from(b"synthetic source context".as_slice()),
        }),
    }
}
fn request(b: &Binding, text: &str) -> ReadyEvaluation {
    ReadyEvaluation {
        binding: b.clone(),
        intent: Arc::from(text.as_bytes()),
    }
}
fn response(b: &Binding) -> ReadyResponse {
    ReadyResponse {
        binding: b.clone(),
        bytes: Arc::from(b"original bounded receipt".as_slice()),
    }
}
fn setup_with(o: Authority) -> (Authority, [Scope; 3], Binding) {
    let w = scope(&o, None);
    let n = scope(&o, Some(&w));
    let r = scope(&o, Some(&n));
    let b = binding(&r);
    o.select_workspace(&w).unwrap();
    (o, [w, n, r], b)
}
fn setup() -> (Authority, [Scope; 3], Binding) {
    setup_with(Authority::from_generation([7; 32]))
}
fn opened() -> (Authority, [Scope; 3], Binding) {
    let (o, s, b) = setup();
    o.register_source_binding(&b).unwrap();
    (o, s, b)
}

#[test]
fn registration_is_required_and_cannot_repeat() {
    let (o, _, b) = setup();
    assert_eq!(o.try_claim(request(&b, "one")).err(), Some(Error::Unissued));
    o.register_source_binding(&b).unwrap();
    assert_eq!(o.register_source_binding(&b), Err(Error::Conflict));
    assert_eq!(
        o.try_claim(request(&b, "one")).unwrap().identity().sequence,
        1
    );
}
#[test]
fn close_before_registration_is_terminal() {
    let (o, _, b) = setup();
    o.close_binding(&b).unwrap();
    assert_eq!(o.register_source_binding(&b), Err(Error::Closed));
    assert_eq!(o.try_claim(request(&b, "late")).err(), Some(Error::Closed));
}
#[test]
fn revoke_first_does_not_consume_a_claim_sequence() {
    let (o, s, b) = opened();
    o.close_binding(&b).unwrap();
    assert_eq!(
        o.try_claim(request(&b, "denied")).err(),
        Some(Error::Closed)
    );
    let next = binding(&s[2]);
    o.register_source_binding(&next).unwrap();
    assert_eq!(
        o.try_claim(request(&next, "new"))
            .unwrap()
            .identity()
            .sequence,
        1
    );
}
#[test]
fn exact_claim_can_settle_after_close_while_close_does_not_wait_for_backend() {
    let (o, _, b) = opened();
    let o = Arc::new(o);
    let claim = o
        .try_claim(request(&b, "exact original evaluation"))
        .unwrap();
    let (parked_tx, parked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker_owner = o.clone();
    let worker = std::thread::spawn(move || {
        parked_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(claim.intent(), b"exact original evaluation");
        worker_owner.settle(&claim, Disposition::Committed)
    });
    parked_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let close_owner = o.clone();
    let closing = b.clone();
    let (done_tx, done_rx) = mpsc::channel();
    let closer = std::thread::spawn(move || {
        done_tx.send(close_owner.close_binding(&closing)).unwrap();
    });
    let closed = done_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    assert_eq!(worker.join().unwrap(), Ok(()));
    closer.join().unwrap();
    assert_eq!(closed.unwrap(), Ok(()));
    assert_eq!(o.try_claim(request(&b, "later")).err(), Some(Error::Closed));
    assert_eq!(o.commit_response(response(&b)).err(), Some(Error::Closed));
}
#[test]
fn stale_close_and_preparation_cannot_alias_replacement_binding() {
    let (o, s, old) = opened();
    let prepared = request(&old, "old");
    o.close_binding(&old).unwrap();
    let replacement = binding(&s[2]);
    o.register_source_binding(&replacement).unwrap();
    o.close_binding(&old).unwrap();
    assert_eq!(o.try_claim(prepared).err(), Some(Error::Closed));
    assert_eq!(
        o.try_claim(request(&replacement, "replacement"))
            .unwrap()
            .identity()
            .sequence,
        1
    );
}
#[test]
fn each_enclosing_scope_closes_claims_registration_and_responses() {
    for index in 0..3 {
        let (o, s, b) = opened();
        o.close_scope(&s[index]).unwrap();
        assert_eq!(
            o.try_claim(request(&b, "closed scope")).err(),
            Some(Error::Closed)
        );
        assert_eq!(o.commit_response(response(&b)).err(), Some(Error::Closed));
        assert_eq!(
            o.register_source_binding(&binding(&s[2])),
            Err(Error::Closed)
        );
    }
}
#[test]
fn sidecar_close_keeps_only_already_claimed_disposition() {
    let (o, _, b) = opened();
    let c = o.try_claim(request(&b, "one")).unwrap();
    o.close().unwrap();
    assert_eq!(o.try_claim(request(&b, "two")).err(), Some(Error::Closed));
    assert_eq!(o.commit_response(response(&b)).err(), Some(Error::Closed));
    assert_eq!(o.settle(&c, Disposition::RolledBack), Ok(()));
    assert_eq!(o.try_claim(request(&b, "three")).err(), Some(Error::Closed));
}
#[test]
fn equal_generation_bytes_do_not_authorize_a_foreign_owner() {
    let (a, sa, ba) = opened();
    let (other, _, bb) = opened();
    assert_eq!(other.register_source_binding(&ba), Err(Error::Foreign));
    assert_eq!(other.close_scope(&sa[0]), Err(Error::Foreign));
    assert_eq!(other.close_binding(&ba), Err(Error::Foreign));
    assert_eq!(
        other.try_claim(request(&ba, "foreign")).err(),
        Some(Error::Foreign)
    );
    assert_eq!(
        other.commit_response(response(&ba)).err(),
        Some(Error::Foreign)
    );
    let claim = a.try_claim(request(&ba, "a")).unwrap();
    assert_eq!(
        other.settle(&claim, Disposition::Committed),
        Err(Error::Foreign)
    );
    assert_eq!(
        other
            .try_claim(request(&bb, "b"))
            .unwrap()
            .identity()
            .sequence,
        1
    );
}
#[test]
fn dropped_claim_does_not_free_the_single_slot() {
    let (o, _, b) = opened();
    drop(o.try_claim(request(&b, "original")).unwrap());
    assert_eq!(
        o.try_claim(request(&b, "replacement")).err(),
        Some(Error::Busy)
    );
}
#[test]
fn unknown_disposition_fences_and_keeps_slot_until_proven_settlement() {
    let (o, _, b) = opened();
    let c = o.try_claim(request(&b, "original")).unwrap();
    assert_eq!(o.settle(&c, Disposition::Unknown), Ok(()));
    assert_eq!(o.try_claim(request(&b, "new")).err(), Some(Error::Busy));
    assert_eq!(
        o.commit_response(response(&b)).err(),
        Some(Error::Unavailable)
    );
    o.settle(&c, Disposition::RolledBack).unwrap();
    assert_eq!(
        o.try_claim(request(&b, "new")).unwrap().identity().sequence,
        2
    );
}
#[test]
fn settled_claim_cannot_retire_a_later_claim() {
    let (o, _, b) = opened();
    let old = o.try_claim(request(&b, "old")).unwrap();
    o.settle(&old, Disposition::Committed).unwrap();
    let new = o.try_claim(request(&b, "new")).unwrap();
    assert_eq!(
        o.settle(&old, Disposition::RolledBack),
        Err(Error::Conflict)
    );
    assert_eq!(o.try_claim(request(&b, "third")).err(), Some(Error::Busy));
    o.settle(&new, Disposition::Committed).unwrap();
}
#[test]
fn changed_token_intent_cannot_settle_the_installed_claim() {
    let (o, _, b) = opened();
    let mut c = o.try_claim(request(&b, "original")).unwrap();
    let original = c.intent.clone();
    c.intent = Arc::from(b"changed target or request".as_slice());
    assert_eq!(o.settle(&c, Disposition::Committed), Err(Error::Conflict));
    assert_eq!(o.try_claim(request(&b, "later")).err(), Some(Error::Busy));
    c.intent = original;
    o.settle(&c, Disposition::RolledBack).unwrap();
}
#[test]
fn old_workspace_claim_settles_without_publishing_into_new_scope() {
    let (o, old, old_binding) = opened();
    let old_prepared = request(&old_binding, "old queued");
    let claim = o.try_claim(request(&old_binding, "old claimed")).unwrap();
    let w = scope(&o, None);
    let n = scope(&o, Some(&w));
    let r = scope(&o, Some(&n));
    let b = binding(&r);
    o.select_workspace(&w).unwrap();
    o.register_source_binding(&b).unwrap();
    assert_eq!(o.try_claim(old_prepared).err(), Some(Error::Closed));
    assert_eq!(
        o.try_claim(request(&b, "new workspace")).err(),
        Some(Error::Busy)
    );
    assert_eq!(
        o.commit_response(response(&b)).err(),
        Some(Error::Unavailable)
    );
    o.settle(&claim, Disposition::Committed).unwrap();
    o.close_scope(&old[0]).unwrap();
    assert_eq!(
        o.commit_response(response(&old_binding)).err(),
        Some(Error::Closed)
    );
    assert_eq!(
        o.try_claim(request(&b, "new workspace"))
            .unwrap()
            .identity()
            .sequence,
        2
    );
}
#[test]
fn closed_new_workspace_stays_closed_after_old_claim_settles() {
    let (o, _, b) = opened();
    let c = o.try_claim(request(&b, "old")).unwrap();
    let w = scope(&o, None);
    o.select_workspace(&w).unwrap();
    o.close_scope(&w).unwrap();
    o.settle(&c, Disposition::Committed).unwrap();
    assert_eq!(o.select_workspace(&w), Err(Error::Closed));
}
#[test]
fn committed_response_keeps_exact_buffer_after_revocation() {
    let (o, _, b) = opened();
    let committed = o.commit_response(response(&b)).unwrap();
    o.close_binding(&b).unwrap();
    assert_eq!(committed.bytes(), b"original bounded receipt");
    assert_eq!(o.commit_response(response(&b)).err(), Some(Error::Closed));
}
#[test]
fn selecting_same_workspace_does_not_close_it() {
    let (o, s, b) = opened();
    o.select_workspace(&s[0]).unwrap();
    assert!(o.try_claim(request(&b, "still selected")).is_ok());
}
#[test]
fn namespace_scope_cannot_be_selected_as_workspace() {
    let (o, s, b) = opened();
    assert_eq!(o.select_workspace(&s[1]), Err(Error::Conflict));
    assert!(o.try_claim(request(&b, "original selected")).is_ok());
}
#[test]
fn sequence_exhaustion_never_wraps_or_reuses_rollback_sequence() {
    let (o, _, b) = opened();
    o.inner.state.lock().unwrap().last_sequence = u64::MAX - 1;
    let c = o.try_claim(request(&b, "last")).unwrap();
    assert_eq!(c.identity().sequence, u64::MAX);
    o.settle(&c, Disposition::RolledBack).unwrap();
    assert_eq!(
        o.try_claim(request(&b, "overflow")).err(),
        Some(Error::Exhausted)
    );
    assert_eq!(
        o.try_claim(request(&b, "retry")).err(),
        Some(Error::Exhausted)
    );
    assert_eq!(o.close_binding(&b), Ok(()));
}
#[test]
fn poisoned_owner_is_unavailable_and_is_not_reinitialized() {
    let (o, _, b) = opened();
    let inner = o.inner.clone();
    assert!(std::thread::spawn(move || {
        let _guard = inner.state.lock().unwrap();
        panic!("injected poison");
    })
    .join()
    .is_err());
    assert_eq!(
        o.try_claim(request(&b, "poisoned")).err(),
        Some(Error::Unavailable)
    );
    assert_eq!(o.close_binding(&b), Err(Error::Unavailable));
    assert_eq!(o.register_source_binding(&b), Err(Error::Unavailable));
}
#[test]
fn entropy_failure_has_no_fallback_authority() {
    assert_eq!(
        Authority::from_entropy(Err(())).err(),
        Some(Error::Unavailable)
    );
}
#[test]
fn exact_supplied_generation_binds_the_claim_identity() {
    let o = Authority::from_entropy(Ok([0xab; 32])).unwrap();
    let (o, _, b) = setup_with(o);
    o.register_source_binding(&b).unwrap();
    let c = o.try_claim(request(&b, "one")).unwrap();
    assert_eq!(
        c.identity(),
        ClaimIdentity {
            sidecar_generation: [0xab; 32],
            sequence: 1
        }
    );
}

#[test]
fn claimed_operation_retains_owner_when_caller_releases_its_reference() {
    let (o, _, b) = opened();
    let claim = o.try_claim(request(&b, "retained operation")).unwrap();
    let retained = Arc::downgrade(&o.inner);
    drop(o);
    let inner = retained
        .upgrade()
        .expect("claimed operation must retain owner context");
    let worker = Authority { inner };
    assert_eq!(
        worker.try_claim(request(&b, "second")).err(),
        Some(Error::Busy)
    );
    worker.settle(&claim, Disposition::RolledBack).unwrap();
}

#[test]
fn simultaneous_requests_install_only_one_claim_and_consume_one_sequence() {
    let (o, _, b) = opened();
    let o = Arc::new(o);
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut attempts = Vec::new();
    for intent in ["first request", "second request"] {
        let o = o.clone();
        let barrier = barrier.clone();
        let ready = request(&b, intent);
        attempts.push(std::thread::spawn(move || {
            barrier.wait();
            o.try_claim(ready)
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = attempts.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(r, Err(Error::Busy)))
            .count(),
        1
    );
    let claim = outcomes
        .into_iter()
        .find_map(std::result::Result::ok)
        .unwrap();
    assert_eq!(claim.identity().sequence, 1);
    o.settle(&claim, Disposition::Committed).unwrap();
    assert_eq!(
        o.try_claim(request(&b, "next"))
            .unwrap()
            .identity()
            .sequence,
        2
    );
}
