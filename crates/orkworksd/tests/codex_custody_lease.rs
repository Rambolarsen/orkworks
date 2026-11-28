#![cfg(target_os = "macos")]
//! Native macOS qualification tests for Step 2: reusable private worker lease,
//! four-slot + descriptor-quota accounting, kernel-bound root transfer, and
//! hung-reader / stale-ACK quarantine against the actual `orkworksd` binary.

#[path = "../src/taskmaster/codex_capture_owner/mod.rs"]
mod codex_capture_owner;

use std::fs;
use std::io::{self, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use codex_capture_owner::channels::{self, SlotChannels};
use codex_capture_owner::framing::{self, BindStatus, ControlFrame, ProbeCommitment};
use codex_capture_owner::lease::{
    DescriptorLedger, LeaseError, LeasePool, LeaseRoots, RevalidateOutcome,
    CONTROLLER_DESCRIPTORS_PER_SLOT, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE, GLOBAL_DESCRIPTOR_CAP,
    MAX_RESOURCE_DESCRIPTORS_PER_SLOT, SLOT_DESCRIPTOR_RESERVATION,
};
use codex_capture_owner::pool::{CustodyPool, OwnedChild, MAX_SLOTS, STARTUP_BUDGET};

fn fd_table_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn orkworksd_bin() -> &'static Path {
    static WARMED: OnceLock<()> = OnceLock::new();
    let bin = Path::new(env!("CARGO_BIN_EXE_orkworksd"));
    WARMED.get_or_init(|| {
        // Trigger macOS dyld/AMFI first-exec scan of a freshly linked debug
        // binary once before asserting against the shared 3-second startup budget.
        let _ = Command::new(bin)
            .arg("--codex-custody-worker")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
    bin
}

struct TempRoots {
    base: PathBuf,
    workspace: PathBuf,
    home: PathBuf,
    packaged: PathBuf,
}

impl TempRoots {
    fn create(label: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "orkworks-custody-lease-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        let workspace = base.join("workspace");
        let home = base.join("home");
        let packaged = base.join("packaged");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&packaged).unwrap();
        Self {
            base,
            workspace,
            home,
            packaged,
        }
    }

    fn write_workspace_probe(&self, contents: &[u8]) {
        fs::write(self.workspace.join("custody-probe.txt"), contents).unwrap();
    }

    fn open(&self) -> LeaseRoots {
        LeaseRoots::open(&self.workspace, &self.home, &self.packaged).unwrap()
    }
}

impl Drop for TempRoots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn process_open_fds(pid: i32) -> io::Result<Vec<RawFd>> {
    // SAFETY: proc_pidinfo with null buffer queries required byte length.
    let bytes_needed =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if bytes_needed < 0 {
        return Err(io::Error::last_os_error());
    }
    let entry_size = std::mem::size_of::<libc::proc_fdinfo>();
    let capacity = (bytes_needed as usize / entry_size).saturating_add(16);
    let mut entries = vec![
        libc::proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        capacity
    ];
    let buf_bytes = (entries.len() * entry_size) as libc::c_int;
    // SAFETY: entries is valid writable storage of `buf_bytes`.
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            entries.as_mut_ptr().cast(),
            buf_bytes,
        )
    };
    if written < 0 {
        return Err(io::Error::last_os_error());
    }
    let count = (written as usize) / entry_size;
    let mut fds: Vec<RawFd> = entries[..count].iter().map(|e| e.proc_fd).collect();
    fds.sort_unstable();
    Ok(fds)
}

#[repr(C)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: libc::off_t,
    fi_type: i32,
    fi_guardflags: u32,
}

#[repr(C)]
struct VnodeFdInfo {
    pfi: ProcFileInfo,
    pvi: libc::vnode_info,
}

const PROC_PIDFDVNODEINFO: libc::c_int = 1;

fn process_has_inode(pid: i32, target_dev: u64, target_ino: u64) -> bool {
    let Ok(fds) = process_open_fds(pid) else {
        return false;
    };
    for fd in fds {
        let mut info = std::mem::MaybeUninit::<VnodeFdInfo>::zeroed();
        // SAFETY: proc_pidfdinfo writes a vnode_fdinfo for vnode descriptors.
        let ret = unsafe {
            libc::proc_pidfdinfo(
                pid,
                fd,
                PROC_PIDFDVNODEINFO,
                info.as_mut_ptr().cast(),
                std::mem::size_of::<VnodeFdInfo>() as libc::c_int,
            )
        };
        if ret == std::mem::size_of::<VnodeFdInfo>() as libc::c_int {
            // SAFETY: proc_pidfdinfo populated the full VnodeFdInfo struct.
            let info = unsafe { info.assume_init() };
            let st = info.pvi.vi_stat;
            if st.vst_dev as u64 == target_dev && st.vst_ino == target_ino {
                return true;
            }
        }
    }
    false
}

fn file_identity(path: &Path) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::symlink_metadata(path).unwrap();
    (meta.dev(), meta.ino())
}

#[test]
fn descriptor_ledger_enforces_per_slot_16_and_global_256_without_borrowing() {
    assert_eq!(SLOT_DESCRIPTOR_RESERVATION, 16);
    assert_eq!(CONTROLLER_DESCRIPTORS_PER_SLOT, 4);
    assert_eq!(MAX_RESOURCE_DESCRIPTORS_PER_SLOT, 12);
    assert_eq!(GLOBAL_DESCRIPTOR_CAP, 256);

    let mut ledger = DescriptorLedger::new();
    for slot in 0..MAX_SLOTS {
        ledger.charge_controller_slot(slot).unwrap();
    }
    assert_eq!(ledger.controller_fds(), 16);
    assert_eq!(ledger.total_fds(), 16);

    // Requesting 13 resource FDs on slot 0 must fail even though slots 1..3 are unused.
    assert_eq!(
        ledger.try_charge_slot_resources(0, 13),
        Err(LeaseError::SlotDescriptorQuotaExceeded)
    );
    assert_eq!(ledger.resource_fds(), 0);

    // Charging up to 12 resource FDs on slot 0 succeeds; adding 1 more fails without borrowing.
    ledger.try_charge_slot_resources(0, 12).unwrap();
    assert_eq!(
        ledger.try_charge_slot_resources(0, 1),
        Err(LeaseError::SlotDescriptorQuotaExceeded)
    );
    assert_eq!(ledger.total_fds(), 28);

    // Global 256 cap counts external + controller + resource + draining charges together.
    ledger.set_external_charge(225).unwrap();
    assert_eq!(ledger.total_fds(), 253);
    // 4 more FDs on slot 1 would exceed 256 (253 + 4 = 257).
    assert_eq!(
        ledger.try_charge_slot_resources(1, 4),
        Err(LeaseError::GlobalDescriptorCapExceeded)
    );
    // 3 more FDs fit exactly at 256.
    ledger.try_charge_slot_resources(1, 3).unwrap();
    assert_eq!(ledger.total_fds(), 256);

    // Release frees only that slot's resource charge; controller remains charged until retired.
    ledger.release_slot_resources(0);
    assert_eq!(ledger.total_fds(), 244);
    ledger.release_controller_slot(0);
    assert_eq!(ledger.total_fds(), 240);
    assert_eq!(ledger.controller_fds(), 12);
}

#[test]
fn bind_observe_revalidate_release_and_reuse_same_worker_without_restart() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("lifecycle");
    temp.write_workspace_probe(b"initial-custody-payload-v1");
    let (probe_dev, probe_ino) = file_identity(&temp.workspace.join("custody-probe.txt"));

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );
    let init = pool.accounting();
    assert_eq!(init.idle, MAX_SLOTS);
    assert_eq!(init.charged_slots, MAX_SLOTS);
    assert_eq!(init.controller_fds, 16);
    assert_eq!(init.resource_fds, 0);

    let roots = temp.open();
    let handle1 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .expect("bind_and_observe on real orkworksd worker");
    let first_pid = handle1.worker_pid();
    let first_expiry = handle1.expires_at();
    assert_eq!(handle1.lease_seq(), 1);
    assert_eq!(handle1.root_identities(), roots.identities());
    assert!(handle1.commitments()[0].present);
    assert_eq!(handle1.commitments()[0].byte_len, 26);

    let bound_acct = pool.accounting();
    assert_eq!(bound_acct.bound, 1);
    assert_eq!(bound_acct.idle, 3);
    assert!(bound_acct.resource_fds > 0);

    // Kernel-bound transfer proof:
    // 1. Parent NEVER opens or receives the synthetic probe file descriptor.
    assert!(
        !process_has_inode(std::process::id() as i32, probe_dev, probe_ino),
        "parent must never hold synthetic resource file descriptors"
    );
    // 2. Worker holds the exact transferred workspace root directory inode and 8 open FDs (5 baseline + 3 roots).
    let ws_id = roots.identities()[0];
    assert!(
        process_has_inode(first_pid as i32, ws_id.dev, ws_id.ino),
        "worker must hold kernel-bound workspace root descriptor while bound"
    );
    let worker_fds_bound = process_open_fds(first_pid as i32).unwrap();
    assert_eq!(
        worker_fds_bound.len(),
        8,
        "bound worker should hold exact 5 baseline + 3 root FDs, got {worker_fds_bound:?}"
    );

    // Revalidate while unchanged -> Fresh, original expiry unchanged.
    assert_eq!(
        pool.revalidate(&handle1, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    assert_eq!(handle1.expires_at(), first_expiry);
    assert!(!process_has_inode(
        std::process::id() as i32,
        probe_dev,
        probe_ino
    ));

    // Explicit release of Fresh lease #1: worker closes all 3 roots BEFORE ReleaseAck.
    pool.release(handle1, DEFAULT_OP_DEADLINE)
        .expect("qualified ReleaseAck");
    let after_release = pool.accounting();
    assert_eq!(after_release.idle, MAX_SLOTS);
    assert_eq!(after_release.bound, 0);
    assert_eq!(after_release.resource_fds, 0);

    // Confirm worker's open FDs returned to the exact 5 baseline entries (close-before-ACK).
    let worker_fds_idle = process_open_fds(first_pid as i32).unwrap();
    assert_eq!(
        worker_fds_idle.len(),
        5,
        "released worker must close all root FDs before ACK, got {worker_fds_idle:?}"
    );
    assert!(
        !process_has_inode(first_pid as i32, ws_id.dev, ws_id.ino),
        "released worker must no longer hold root descriptor"
    );

    // Reuse: next bind reuses the EXACT SAME worker PID on slot 0 with lease_seq == 2.
    let handle2 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .expect("reuse same worker without restart");
    assert_eq!(handle2.slot(), 0);
    assert_eq!(handle2.worker_pid(), first_pid);
    assert_eq!(handle2.lease_seq(), 2);

    // Mutate workspace probe file -> Revalidate returns Changed, immediately invalidates
    // preview usability (even if file is reverted), and drains worker back to Idle.
    temp.write_workspace_probe(b"mutated-custody-payload-v2!");
    assert_eq!(
        pool.revalidate(&handle2, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Changed
    );
    assert!(!pool.is_handle_usable(&handle2));
    temp.write_workspace_probe(b"initial-custody-payload-v1");
    assert_eq!(
        pool.revalidate(&handle2, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    assert_eq!(pool.accounting().resource_fds, 0);
    assert_eq!(process_open_fds(first_pid as i32).unwrap().len(), 5);

    // Third bind still reuses the exact same worker PID with lease_seq == 3.
    let handle3 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(handle3.slot(), 0);
    assert_eq!(handle3.worker_pid(), first_pid);
    assert_eq!(handle3.lease_seq(), 3);
    pool.release(handle3, DEFAULT_OP_DEADLINE).unwrap();

    // Symlink at probe path must be rejected (O_NOFOLLOW) while leaving worker healthy & idle.
    let symlink_temp = TempRoots::create("symlink-reject");
    let target = symlink_temp.base.join("outside.txt");
    fs::write(&target, b"outside").unwrap();
    std::os::unix::fs::symlink(&target, symlink_temp.workspace.join("custody-probe.txt")).unwrap();
    let symlink_roots = symlink_temp.open();
    assert_eq!(
        pool.bind_and_observe(&symlink_roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE),
        Err(LeaseError::ObservationRejected)
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    assert_eq!(pool.accounting().resource_fds, 0);

    // Oversized probe (> 256 KiB) must be rejected without crashing or leaking worker roots.
    let oversize_temp = TempRoots::create("oversize-reject");
    oversize_temp.write_workspace_probe(&vec![b'x'; 256 * 1024 + 1]);
    let oversize_roots = oversize_temp.open();
    assert_eq!(
        pool.bind_and_observe(&oversize_roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE),
        Err(LeaseError::ObservationRejected)
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    assert_eq!(pool.accounting().resource_fds, 0);

    // Global 256-FD cap on the live pool rejects bind without evicting or crashing workers.
    pool.set_external_fd_charge_for_test(235).unwrap(); // 16 controller + 235 = 251; +7 bind > 256.
    assert_eq!(
        pool.bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE),
        Err(LeaseError::GlobalDescriptorCapExceeded)
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    pool.set_external_fd_charge_for_test(0).unwrap();

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
    assert_eq!(report.cleanup_unconfirmed, 0);
    assert_eq!(pool.accounting().total_fds, 0);
}

#[test]
fn nonblocking_saturation_denies_fifth_lease_without_queue_or_eviction() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("saturation");
    temp.write_workspace_probe(b"sat-check");
    let roots = temp.open();

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    let mut handles = Vec::new();
    for _ in 0..MAX_SLOTS {
        handles.push(
            pool.bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
                .unwrap(),
        );
    }
    let acct = pool.accounting();
    assert_eq!(acct.bound, MAX_SLOTS);
    assert_eq!(acct.idle, 0);
    assert_eq!(acct.charged_slots, MAX_SLOTS);

    // 5th acquisition fails immediately (nonblocking, no queue, no eviction).
    let start = Instant::now();
    let fifth = pool.bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE);
    assert_eq!(fifth, Err(LeaseError::Unavailable));
    assert!(
        start.elapsed() < Duration::from_millis(200),
        "saturated acquire must not block"
    );

    // All 4 existing leases remain valid and Fresh.
    for h in &handles {
        assert_eq!(
            pool.revalidate(h, DEFAULT_OP_DEADLINE).unwrap(),
            RevalidateOutcome::Fresh
        );
    }

    // Releasing one slot allows reuse of that exact worker PID.
    let freed = handles.remove(1);
    let freed_pid = freed.worker_pid();
    pool.release(freed, DEFAULT_OP_DEADLINE).unwrap();
    let replacement = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(replacement.worker_pid(), freed_pid);
    handles.push(replacement);

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
    // After shutdown, all previously issued handles are invalidated.
    for h in &handles {
        assert!(!pool.is_handle_usable(h));
        assert_eq!(
            pool.revalidate(h, DEFAULT_OP_DEADLINE),
            Err(LeaseError::InvalidHandle)
        );
    }
}

#[test]
fn forged_stale_and_cross_pool_handles_are_rejected() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("forged");
    temp.write_workspace_probe(b"forged-check");
    let roots = temp.open();

    let mut pool_a = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );
    let mut pool_b = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    let handle_a = pool_a
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    let handle_b = pool_b
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();

    // Cross-pool (restart / different sidecar) handle must be rejected.
    assert!(!pool_a.is_handle_usable(&handle_b));
    assert_eq!(
        pool_a.revalidate(&handle_b, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );
    assert_eq!(
        pool_a.release(handle_b.clone(), DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    // Forged lease_id, lease_seq, and worker_boot_nonce must be rejected.
    let mut bad_id = handle_a.clone();
    bad_id.corrupt_lease_id_for_test();
    assert_eq!(
        pool_a.revalidate(&bad_id, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    let mut bad_seq = handle_a.clone();
    bad_seq.corrupt_lease_seq_for_test();
    assert_eq!(
        pool_a.revalidate(&bad_seq, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    let mut bad_nonce = handle_a.clone();
    bad_nonce.corrupt_boot_nonce_for_test();
    assert_eq!(
        pool_a.revalidate(&bad_nonce, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    // Original live handle is still healthy; after release, stale copy is rejected.
    let stale_copy = handle_a.clone();
    assert_eq!(
        pool_a.revalidate(&handle_a, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    pool_a.release(handle_a, DEFAULT_OP_DEADLINE).unwrap();
    assert_eq!(
        pool_a.revalidate(&stale_copy, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );

    let _ = pool_a.shutdown(Duration::from_secs(2));
    let _ = pool_b.shutdown(Duration::from_secs(2));
}

#[test]
fn checked_sequence_overflow_retires_slot_permanently() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("seq-overflow");
    temp.write_workspace_probe(b"seq-overflow-payload");
    let roots = temp.open();

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    // 1. Lease sequence exhaustion: bind with lease_seq == u64::MAX, release -> slot 0 retires cleanly.
    pool.set_slot_next_lease_seq_for_test(0, Some(u64::MAX));
    let h0 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(h0.slot(), 0);
    assert_eq!(h0.lease_seq(), u64::MAX);
    assert_eq!(
        pool.revalidate(&h0, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    pool.release(h0, DEFAULT_OP_DEADLINE).unwrap();

    let acct_after_lease_overflow = pool.accounting();
    assert_eq!(acct_after_lease_overflow.retired, 1);
    assert_eq!(acct_after_lease_overflow.idle, 3);
    assert_eq!(acct_after_lease_overflow.charged_slots, 3);
    assert_eq!(acct_after_lease_overflow.controller_fds, 12);

    // 2. Op sequence exhaustion on slot 1: next revalidate fails closed and permanently retires slot 1.
    let h1 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(h1.slot(), 1);
    pool.set_slot_last_op_seq_for_test(1, u64::MAX);
    assert_eq!(
        pool.revalidate(&h1, DEFAULT_OP_DEADLINE),
        Err(LeaseError::SequenceExhausted)
    );
    assert!(!pool.is_handle_usable(&h1));

    let acct_after_op_overflow = pool.accounting();
    assert_eq!(acct_after_op_overflow.retired, 2);
    assert_eq!(acct_after_op_overflow.idle, 2);
    assert_eq!(acct_after_op_overflow.charged_slots, 2);

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, 2);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

#[test]
fn expired_lease_refuses_revalidate_invalidates_first_and_drains() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("expiry");
    temp.write_workspace_probe(b"expiry-check");
    let roots = temp.open();

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    // 1. Parent-enforced expiry: invalidates immediately, refuses Revalidate, drains slot back to Idle.
    let handle = pool
        .bind_and_observe(&roots, Duration::from_millis(80), DEFAULT_OP_DEADLINE)
        .unwrap();
    let pid = handle.worker_pid();
    thread::sleep(Duration::from_millis(120));
    assert_eq!(
        pool.revalidate(&handle, DEFAULT_OP_DEADLINE),
        Err(LeaseError::Expired)
    );
    assert!(!pool.is_handle_usable(&handle));
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    assert_eq!(pool.accounting().resource_fds, 0);

    // Worker process was cleanly drained and is reusable.
    let reused = pool
        .bind_and_observe(&roots, Duration::from_millis(80), DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(reused.worker_pid(), pid);

    // 2. Worker local defense-in-depth timer: extend ONLY parent view past worker TTL
    // to prove the worker itself enforces its original TTL and never renews it.
    pool.extend_parent_expiry_for_test(reused.slot(), Duration::from_secs(10));
    thread::sleep(Duration::from_millis(120));
    assert_eq!(
        pool.revalidate(&reused, DEFAULT_OP_DEADLINE),
        Err(LeaseError::Expired)
    );
    assert!(!pool.is_handle_usable(&reused));
    assert_eq!(pool.accounting().idle, MAX_SLOTS);

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
}

#[test]
fn cancel_invalidates_immediately_and_drains_worker_to_idle() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("cancel");
    temp.write_workspace_probe(b"cancel-check");
    let roots = temp.open();

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    let handle = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    let copy = handle.clone();
    pool.cancel_and_drain(handle, DEFAULT_OP_DEADLINE).unwrap();
    assert!(!pool.is_handle_usable(&copy));
    assert_eq!(
        pool.revalidate(&copy, DEFAULT_OP_DEADLINE),
        Err(LeaseError::InvalidHandle)
    );
    assert_eq!(pool.accounting().idle, MAX_SLOTS);
    assert_eq!(pool.accounting().resource_fds, 0);

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, MAX_SLOTS);
}

#[test]
fn hung_real_worker_during_bind_revalidate_and_release_is_terminated_reaped_and_retired() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("hung");
    temp.write_workspace_probe(b"hung-check");
    let roots = temp.open();

    let mut pool = LeasePool::start_for_executable(
        orkworksd_bin(),
        codex_capture_owner::build_identity(),
        STARTUP_BUDGET,
    );

    // 1. Hang worker 0 with SIGSTOP before BindAndObserve -> timeout, SIGKILL + reap, permanent retire.
    let pid0 = pool.slot_pid(0).unwrap();
    // SAFETY: pid0 is our owned child process.
    assert_eq!(unsafe { libc::kill(pid0 as i32, libc::SIGSTOP) }, 0);
    let err0 = pool.bind_and_observe(&roots, DEFAULT_LEASE_TTL, Duration::from_millis(150));
    assert!(
        matches!(err0, Err(LeaseError::OperationTimedOut)),
        "expected timeout on stopped worker 0, got {err0:?}"
    );
    assert_eq!(pool.accounting().retired, 1);
    assert_eq!(pool.accounting().idle, 3);
    assert_eq!(pool.accounting().resource_fds, 0);

    // 2. Bind worker 1, then SIGSTOP before Revalidate -> invalidate first, timeout, SIGKILL + reap, retire.
    let h1 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(h1.slot(), 1);
    let pid1 = h1.worker_pid();
    // SAFETY: pid1 is our owned child process.
    assert_eq!(unsafe { libc::kill(pid1 as i32, libc::SIGSTOP) }, 0);
    let err1 = pool.revalidate(&h1, Duration::from_millis(150));
    assert!(
        matches!(err1, Err(LeaseError::OperationTimedOut)),
        "expected timeout on stopped worker 1 revalidate, got {err1:?}"
    );
    assert!(!pool.is_handle_usable(&h1));
    assert_eq!(pool.accounting().retired, 2);
    assert_eq!(pool.accounting().idle, 2);
    assert_eq!(pool.accounting().resource_fds, 0);

    // 3. Bind worker 2, then SIGSTOP before Release -> invalidate first, timeout, SIGKILL + reap, retire.
    let h2 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(h2.slot(), 2);
    let h2_copy = h2.clone();
    let pid2 = h2.worker_pid();
    // SAFETY: pid2 is our owned child process.
    assert_eq!(unsafe { libc::kill(pid2 as i32, libc::SIGSTOP) }, 0);
    let err2 = pool.release(h2, Duration::from_millis(150));
    assert!(
        matches!(err2, Err(LeaseError::OperationTimedOut)),
        "expected timeout on stopped worker 2 release, got {err2:?}"
    );
    assert!(!pool.is_handle_usable(&h2_copy));
    assert_eq!(pool.accounting().retired, 3);
    assert_eq!(pool.accounting().idle, 1);
    assert_eq!(pool.accounting().resource_fds, 0);

    // 4. Surviving slot 3 remains completely functional and never replaced any retired slot.
    let h3 = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(h3.slot(), 3);
    assert_eq!(
        pool.revalidate(&h3, DEFAULT_OP_DEADLINE).unwrap(),
        RevalidateOutcome::Fresh
    );
    pool.release(h3, DEFAULT_OP_DEADLINE).unwrap();

    let report = pool.shutdown(Duration::from_secs(2));
    assert_eq!(report.clean_exits, 1);
    assert_eq!(report.cleanup_unconfirmed, 0);
}

#[test]
fn stale_release_ack_is_rejected_and_quarantines_worker_permanently() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("stale-ack");
    temp.write_workspace_probe(b"stale-ack-check");
    let roots = temp.open();
    let expected_ids = roots.identities();
    let build = codex_capture_owner::build_identity();

    // Spawn 1 synthetic misbehaving worker on slot 0 (real sleep child + in-process
    // peer thread that speaks the real wire protocol but replies to Release with a
    // stale op_seq) and 3 retired slots.
    let (peer_handles_tx, peer_handles_rx) = std::sync::mpsc::channel();
    let custody = CustodyPool::start_with_slot_hook(
        build,
        Duration::from_secs(3),
        Duration::from_millis(250),
        move |slot, request_rx_fd, response_tx_fd| {
            if slot != 0 {
                return Err(io::Error::other("unused test slot"));
            }
            // Dup the endpoints before the hook returns (pool closes original child ends).
            // SAFETY: request_rx_fd and response_tx_fd are open during hook invocation.
            let dup_req = unsafe { libc::fcntl(request_rx_fd, libc::F_DUPFD_CLOEXEC, 3) };
            let dup_res = unsafe { libc::fcntl(response_tx_fd, libc::F_DUPFD_CLOEXEC, 3) };
            assert!(dup_req >= 3 && dup_res >= 3);
            // SAFETY: dup_req and dup_res are newly allocated FDs owned here.
            let req_stream = UnixStream::from(unsafe { OwnedFd::from_raw_fd(dup_req) });
            let res_fd = unsafe { OwnedFd::from_raw_fd(dup_res) };

            let child = Command::new("/bin/sleep")
                .arg("30")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let pid = child.id() as libc::pid_t;
            // Leak Child drop ownership into OwnedChild so OwnedChild owns waitpid/SIGKILL.
            std::mem::forget(child);
            let owned_child = OwnedChild::try_from_pid(pid)?;

            let join = thread::spawn(move || {
                // 1. Complete real 54-byte handshake.
                let challenge = framing::read_bootstrap_frame(
                    req_stream.as_fd(),
                    Instant::now() + Duration::from_secs(2),
                )
                .unwrap();
                let mut ready = challenge;
                ready[5] = 2; // Direction::Ready
                framing::write_bootstrap_frame(
                    res_fd.as_fd(),
                    &ready,
                    Instant::now() + Duration::from_secs(2),
                )
                .unwrap();

                // 2. Accept BindAndObserve with 3 FDs and reply BindObserved.
                let (frame, fds) = framing::recv_request_frame_or_eof(
                    req_stream.as_fd(),
                    Some(Instant::now() + Duration::from_secs(2)),
                )
                .unwrap()
                .unwrap();
                assert_eq!(fds.len(), 3);
                let ControlFrame::BindAndObserve {
                    lease_id,
                    lease_seq,
                    op_seq,
                    ..
                } = frame
                else {
                    panic!("expected BindAndObserve");
                };
                framing::write_control_frame(
                    res_fd.as_fd(),
                    &ControlFrame::BindObserved {
                        lease_id,
                        lease_seq,
                        op_seq,
                        status: BindStatus::Observed,
                        root_identities: expected_ids,
                        probe_commitments: [ProbeCommitment::absent(); 3],
                    },
                    Instant::now() + Duration::from_secs(2),
                )
                .unwrap();

                // 3. On Release, reply with STALE op_seq (op_seq - 1) — must be rejected!
                let (rel_frame, rel_fds) = framing::recv_request_frame_or_eof(
                    req_stream.as_fd(),
                    Some(Instant::now() + Duration::from_secs(2)),
                )
                .unwrap()
                .unwrap();
                assert!(rel_fds.is_empty());
                let ControlFrame::Release {
                    lease_id,
                    lease_seq,
                    op_seq,
                } = rel_frame
                else {
                    panic!("expected Release");
                };
                let _ = framing::write_control_frame(
                    res_fd.as_fd(),
                    &ControlFrame::ReleaseAck {
                        lease_id,
                        lease_seq,
                        op_seq: op_seq - 1, // STALE!
                    },
                    Instant::now() + Duration::from_secs(2),
                );
                // Keep channel endpoints open until the controller quarantines and
                // closes the channel, just like a live persistent worker.
                let _ = framing::recv_request_frame_or_eof(
                    req_stream.as_fd(),
                    Some(Instant::now() + Duration::from_secs(2)),
                );
                drop(fds);
            });
            peer_handles_tx.send(join).unwrap();
            Ok(owned_child)
        },
    );

    let mut pool = LeasePool::from_custody_pool(custody);
    assert_eq!(pool.accounting().idle, 1);

    let handle = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    let err = pool.release(handle, DEFAULT_OP_DEADLINE);
    assert_eq!(err, Err(LeaseError::StaleReleaseAck));

    // Slot 0 must be permanently retired, NEVER returned to Idle!
    let acct = pool.accounting();
    assert_eq!(acct.idle, 0);
    assert_eq!(acct.bound, 0);
    assert_eq!(acct.retired, MAX_SLOTS);
    assert_eq!(acct.total_fds, 0);

    peer_handles_rx.recv().unwrap().join().unwrap();
}

#[test]
fn real_worker_rejects_wrong_fd_count_and_non_directory_root_fail_closed() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("bad-roots");
    let regular_file = temp.base.join("not-a-dir.txt");
    fs::write(&regular_file, b"regular").unwrap();
    let dir_a = fs::File::open(&temp.workspace).unwrap();
    let dir_b = fs::File::open(&temp.home).unwrap();
    let reg_c = fs::File::open(&regular_file).unwrap();

    // Helper: spawn one real orkworksd worker on a fresh split channel and complete handshake.
    let spawn_handshaken = || -> (UnixStream, OwnedFd, OwnedChild) {
        let [SlotChannels {
            request_tx,
            request_rx,
            response_rx,
            response_tx,
        }, _, _, _] = channels::seal_split_channels().unwrap();
        let child = codex_capture_owner::pool::spawn_worker_process_for_test(
            orkworksd_bin(),
            request_rx.as_raw_fd(),
            response_tx.as_raw_fd(),
        )
        .unwrap();
        drop((request_rx, response_tx));
        let nonce = *uuid::Uuid::new_v4().as_bytes();
        let build = codex_capture_owner::build_identity();
        let deadline = Instant::now() + Duration::from_secs(3);
        let challenge = codex_capture_owner::encode_challenge_for_test(&nonce, &build);
        framing::write_bootstrap_frame(request_tx.as_fd(), &challenge, deadline).unwrap();
        let _ready = framing::read_bootstrap_frame(response_rx.as_fd(), deadline).unwrap();
        (request_tx, response_rx, child)
    };

    // Case 1: BindAndObserve with only 2 FDs instead of 3 -> worker fails closed and exits.
    {
        let (req_tx, res_rx, mut child) = spawn_handshaken();
        let frame = ControlFrame::BindAndObserve {
            lease_id: *uuid::Uuid::new_v4().as_bytes(),
            lease_seq: 1,
            op_seq: 1,
            ttl_ms: 5_000,
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        framing::send_control_frame_with_fds(
            req_tx.as_fd(),
            &frame,
            &[dir_a.as_raw_fd(), dir_b.as_raw_fd()],
            deadline,
        )
        .unwrap();
        let reply = framing::read_control_frame(res_rx.as_fd(), deadline);
        assert!(reply.is_err(), "worker must fail closed on 2 FDs");
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    // Case 2: BindAndObserve with a regular file as the 3rd FD (not S_IFDIR) -> worker fails closed.
    {
        let (req_tx, res_rx, mut child) = spawn_handshaken();
        let frame = ControlFrame::BindAndObserve {
            lease_id: *uuid::Uuid::new_v4().as_bytes(),
            lease_seq: 1,
            op_seq: 1,
            ttl_ms: 5_000,
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        framing::send_control_frame_with_fds(
            req_tx.as_fd(),
            &frame,
            &[dir_a.as_raw_fd(), dir_b.as_raw_fd(), reg_c.as_raw_fd()],
            deadline,
        )
        .unwrap();
        let reply = framing::read_control_frame(res_rx.as_fd(), deadline);
        assert!(
            reply.is_err(),
            "worker must fail closed on non-directory FD"
        );
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    // Case 3: Unexpected SCM_RIGHTS attached to Ping -> worker fails closed and exits.
    {
        let (req_tx, res_rx, mut child) = spawn_handshaken();
        let frame = ControlFrame::Ping(*uuid::Uuid::new_v4().as_bytes());
        let deadline = Instant::now() + Duration::from_secs(2);
        framing::send_control_frame_with_fds(
            req_tx.as_fd(),
            &frame,
            &[dir_a.as_raw_fd()],
            deadline,
        )
        .unwrap();
        let reply = framing::read_control_frame(res_rx.as_fd(), deadline);
        assert!(
            reply.is_err(),
            "worker must fail closed on unexpected FD on Ping"
        );
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    // Case 4: BindAndObserve with 4 FDs (> 3) -> worker fails closed and exits.
    {
        let (req_tx, res_rx, mut child) = spawn_handshaken();
        let frame = ControlFrame::BindAndObserve {
            lease_id: *uuid::Uuid::new_v4().as_bytes(),
            lease_seq: 1,
            op_seq: 1,
            ttl_ms: 5_000,
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        framing::send_control_frame_with_fds(
            req_tx.as_fd(),
            &frame,
            &[
                dir_a.as_raw_fd(),
                dir_b.as_raw_fd(),
                dir_a.as_raw_fd(),
                dir_b.as_raw_fd(),
            ],
            deadline,
        )
        .unwrap();
        let reply = framing::read_control_frame(res_rx.as_fd(), deadline);
        assert!(reply.is_err(), "worker must fail closed on 4 FDs");
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    // Case 5: BindAndObserve with 9 FDs (> MAX_RECV_CMSG_FDS = 8, triggering kernel MSG_CTRUNC)
    // -> worker fails closed and exits cleanly without claiming stdio or leaking.
    {
        let (req_tx, res_rx, mut child) = spawn_handshaken();
        let frame = ControlFrame::BindAndObserve {
            lease_id: *uuid::Uuid::new_v4().as_bytes(),
            lease_seq: 1,
            op_seq: 1,
            ttl_ms: 5_000,
        };
        let encoded = framing::encode_control_frame(&frame);
        let nine_fds = [dir_a.as_raw_fd(); 9];
        send_raw_scm_rights_unchecked(req_tx.as_raw_fd(), &encoded, &nine_fds).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let reply = framing::read_control_frame(res_rx.as_fd(), deadline);
        assert!(reply.is_err(), "worker must fail closed on MSG_CTRUNC");
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    // Keep temp files alive through tests.
    let _ = (&dir_a, &dir_b, &reg_c, Write::flush(&mut io::sink()));
}

fn send_raw_scm_rights_unchecked(sock_fd: RawFd, payload: &[u8], fds: &[RawFd]) -> io::Result<()> {
    let fds_bytes = std::mem::size_of_val(fds) as libc::c_uint;
    // SAFETY: CMSG_SPACE computes aligned control buffer size for 9 FDs.
    let cmsg_space = unsafe { libc::CMSG_SPACE(fds_bytes) } as usize;
    let mut cmsg_buf = vec![0u64; cmsg_space.div_ceil(std::mem::size_of::<u64>())];
    let mut iov = libc::iovec {
        iov_base: payload.as_ptr() as *mut libc::c_void,
        iov_len: payload.len(),
    };
    // SAFETY: zero-initialized msghdr populated before sendmsg.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_space as libc::socklen_t;
    // SAFETY: cmsg_buf has cmsg_space aligned writable bytes.
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(fds_bytes);
        std::ptr::copy_nonoverlapping(
            fds.as_ptr().cast::<u8>(),
            libc::CMSG_DATA(cmsg),
            fds_bytes as usize,
        );
    }
    // SAFETY: sock_fd is an open AF_UNIX stream and msg points to live buffers.
    let sent = unsafe { libc::sendmsg(sock_fd, &msg, 0) };
    if sent <= 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[test]
fn booting_and_cleanup_unconfirmed_slots_retain_charges_until_reconciled() {
    let _guard = fd_table_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempRoots::create("reconcile");
    temp.write_workspace_probe(b"reconcile-check");
    let roots = temp.open();
    let bin = orkworksd_bin().to_path_buf();
    let build = codex_capture_owner::build_identity();

    // Slot 0 spawns a real worker immediately; slot 1 blocks behind a gate past the
    // startup deadline so it remains Booting (PendingSpawn) with 4 controller FDs charged.
    let gate = std::sync::Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let gate_hook = std::sync::Arc::clone(&gate);
    let bin_hook = bin.clone();
    let custody = CustodyPool::start_with_slot_hook(
        build,
        Duration::from_millis(450),
        Duration::from_millis(150),
        move |slot, req_rx, res_tx| {
            if slot == 0 {
                return codex_capture_owner::pool::spawn_worker_process_for_test(
                    &bin_hook, req_rx, res_tx,
                );
            }
            if slot == 1 {
                let (lock, cvar) = &*gate_hook;
                let mut open = lock.lock().unwrap();
                while !*open {
                    open = cvar.wait(open).unwrap();
                }
                return codex_capture_owner::pool::spawn_worker_process_for_test(
                    &bin_hook, req_rx, res_tx,
                );
            }
            Err(io::Error::other("unused test slot"))
        },
    );

    let mut pool = LeasePool::from_custody_pool(custody);
    let init = pool.accounting();
    assert_eq!(init.idle, 1);
    assert_eq!(init.booting, 1);
    assert_eq!(init.retired, 2);
    assert_eq!(init.charged_slots, 2);
    assert_eq!(init.controller_fds, 8);

    // Release the late-spawn gate and reconcile the Booting slot nonblockingly.
    {
        let (lock, cvar) = &*gate;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while pool.accounting().booting > 0 || pool.accounting().cleanup_unconfirmed > 0 {
        let _ = pool.reconcile_unconfirmed();
        assert!(
            Instant::now() < deadline,
            "timed out reconciling late Booting slot: {:?}",
            pool.accounting()
        );
        thread::sleep(Duration::from_millis(10));
    }
    let after_boot_reconcile = pool.accounting();
    assert_eq!(after_boot_reconcile.idle, 1);
    assert_eq!(after_boot_reconcile.booting, 0);
    assert_eq!(after_boot_reconcile.retired, 3);
    assert_eq!(after_boot_reconcile.controller_fds, 4);

    // Now bind slot 0 (charging 6 resource FDs) and quarantine with zero wait budget
    // so if cleanup is initially unconfirmed, BOTH controller (4) and resource (6) FDs
    // stay charged until reconcile_unconfirmed() confirms reap.
    let _handle = pool
        .bind_and_observe(&roots, DEFAULT_LEASE_TTL, DEFAULT_OP_DEADLINE)
        .unwrap();
    assert_eq!(pool.accounting().bound, 1);
    assert_eq!(pool.accounting().resource_fds, 6);
    pool.quarantine_slot_for_test(0, Duration::ZERO);
    let after_zero_wait = pool.accounting();
    if after_zero_wait.cleanup_unconfirmed == 1 {
        assert_eq!(after_zero_wait.controller_fds, 4);
        assert_eq!(after_zero_wait.resource_fds, 6);
        assert_eq!(after_zero_wait.total_fds, 10);
    }

    let reap_deadline = Instant::now() + Duration::from_secs(2);
    while pool.accounting().cleanup_unconfirmed > 0 {
        let _ = pool.reconcile_unconfirmed();
        assert!(Instant::now() < reap_deadline);
        thread::sleep(Duration::from_millis(10));
    }
    let final_acct = pool.accounting();
    assert_eq!(final_acct.retired, MAX_SLOTS);
    assert_eq!(final_acct.controller_fds, 0);
    assert_eq!(final_acct.resource_fds, 0);
    assert_eq!(final_acct.total_fds, 0);
}
