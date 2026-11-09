//! Private local coordinator. No production registration/Open factory exists.
pub(in crate::runtime) use super::recipe::Job;
use super::recipe::Recipe;
use super::scope::{OriginalInputScope, ScopeBudget, ScopeError, ScopeLimits, ScopeReservation};
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::runtime) struct Charges {
    pub bytes: usize,
    pub handles: usize,
    pub notifications: usize,
}
impl Charges {
    fn add(self, other: Self) -> Option<Self> {
        Some(Self {
            bytes: self.bytes.checked_add(other.bytes)?,
            handles: self.handles.checked_add(other.handles)?,
            notifications: self.notifications.checked_add(other.notifications)?,
        })
    }
    fn fits(self, limit: Self) -> bool {
        self.bytes <= limit.bytes
            && self.handles <= limit.handles
            && self.notifications <= limit.notifications
    }
}
#[derive(Clone, Copy)]
pub(in crate::runtime) struct Profile {
    pub records: Limits,
    pub roots: usize,
    pub root_backing_bytes: usize,
    pub scopes: ScopeLimits,
    /// Declared recipe charges only, not a physical payload/native fit profile.
    pub limit: Charges,
    pub per_job: Charges,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::runtime) struct RouteKey {
    pub index: usize,
    pub serial: u64,
}
struct Registration {
    key: RouteKey,
    root: u128,
    incarnation: u64,
    scope: Option<OriginalInputScope>,
    closing: bool,
    jobs: usize,
}
pub(in crate::runtime) struct Registry {
    profile: Profile,
    roots: Vec<Option<Registration>>,
    serial: u64,
    used: Charges,
    // No production API can supply missing native/module/profile qualification.
    fixture_supported: bool,
}
impl Registry {
    pub(in crate::runtime) fn release(&mut self, recipe: &Recipe) {
        let root = self.roots[recipe.route.index]
            .as_mut()
            .expect("retained original route");
        assert!(root.key == recipe.route);
        root.jobs -= 1;
        self.used.bytes -= recipe.charge.bytes;
        self.used.handles -= recipe.charge.handles;
        self.used.notifications -= recipe.charge.notifications;
    }
    fn route(&self, key: RouteKey) -> Result<&Registration, Error> {
        self.roots
            .get(key.index)
            .and_then(Option::as_ref)
            .filter(|r| r.key == key && r.scope.is_some())
            .ok_or(Error::Stale)
    }
}
/// Routing reference only: not native RootKey, readiness or input authority.
pub(in crate::runtime) struct Route {
    core: Arc<Core>,
    key: RouteKey,
}
pub(in crate::runtime) struct Coordinator {
    owner: Owner,
    scopes: ScopeBudget,
}
pub(in crate::runtime) struct CoordinatedSubmission {
    pub submission: Submission,
    pub job: Job,
}

impl Drop for Route {
    fn drop(&mut self) {
        if let Ok(mut state) = self.core.lock() {
            if let Some(registry) = state.coordinator.as_mut() {
                if let Some(root) = registry
                    .roots
                    .get_mut(self.key.index)
                    .and_then(Option::as_mut)
                    .filter(|root| root.key == self.key)
                {
                    if root.scope.is_none() {
                        // Removed routing handles still occupy capacity until Drop.
                        // The scope was already released outside the transition lock.
                        registry.roots[self.key.index] = None;
                    } else {
                        // Lost routing ownership is not successful unregistration.
                        root.closing = true;
                    }
                }
            }
        }
    }
}

impl Coordinator {
    pub(in crate::runtime) fn new(profile: Profile) -> Result<Self, Error> {
        let root_bytes = profile
            .roots
            .checked_mul(std::mem::size_of::<Option<Registration>>())
            .ok_or(Error::InvalidLimits)?;
        let scope_count = profile
            .roots
            .checked_add(profile.records.records)
            .ok_or(Error::InvalidLimits)?;
        if profile.roots == 0
            || root_bytes > profile.root_backing_bytes
            || scope_count > profile.scopes.scopes
            || profile.per_job.bytes == 0
            || profile.per_job.handles == 0
            || profile.per_job.notifications == 0
            || !profile.per_job.fits(profile.limit)
            || profile
                .records
                .backing_bytes
                .checked_add(profile.root_backing_bytes)
                .and_then(|n| n.checked_add(profile.scopes.backing_bytes))
                .and_then(|n| n.checked_add(profile.limit.bytes))
                .is_none()
        {
            return Err(Error::InvalidLimits);
        }
        let scopes = ScopeBudget::new(profile.scopes).map_err(|_| Error::InvalidLimits)?;
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(profile.roots)
            .map_err(|_| Error::Unavailable)?;
        if roots
            .capacity()
            .checked_mul(std::mem::size_of::<Option<Registration>>())
            .is_none_or(|n| n > profile.root_backing_bytes)
        {
            return Err(Error::InvalidLimits);
        }
        roots.resize_with(profile.roots, || None);
        let owner = Owner::new(profile.records)?;
        owner.core.lock()?.coordinator = Some(Registry {
            profile,
            roots,
            serial: 0,
            used: Charges::default(),
            fixture_supported: false,
        });
        Ok(Self { owner, scopes })
    }
    pub(in crate::runtime) fn reserve_scope(&self) -> Result<ScopeReservation, ScopeError> {
        self.scopes.reserve()
    }
    pub(in crate::runtime) fn observe(&self, context: Context) -> Result<Observer, Error> {
        self.owner.observe(context)
    }
    pub(in crate::runtime) fn retire(&self, observer: Observer) -> Result<Snapshot, Error> {
        self.owner.retire(observer)
    }

    pub(in crate::runtime) fn admit(
        &self,
        route: &Route,
        context: Context,
        bytes: usize,
        deadline: Instant,
        scope: OriginalInputScope,
    ) -> Result<CoordinatedSubmission, BoundRefusal> {
        let mut scope = Some(scope);
        let result = (|| {
            if !Arc::ptr_eq(&self.owner.core, &route.core)
                || !self.scopes.owns(scope.as_ref().unwrap())
            {
                return Err(Error::Stale);
            }
            let mut state = self.owner.core.lock()?;
            let registry = state.coordinator.as_ref().ok_or(Error::Unsupported)?;
            if !registry.fixture_supported {
                return Err(Error::Unsupported);
            }
            let root = registry.route(route.key)?;
            if root.root != context.root
                || root.incarnation != context.incarnation
                || !root
                    .scope
                    .as_ref()
                    .unwrap()
                    .same_binding(scope.as_ref().unwrap())
            {
                return Err(Error::Stale);
            }
            if root.closing {
                return Err(Error::Closing);
            }
            let charge = registry.profile.per_job;
            let used = registry
                .used
                .add(charge)
                .filter(|c| c.fits(registry.profile.limit))
                .ok_or(Error::Capacity)?;
            let jobs = root.jobs.checked_add(1).ok_or(Error::Capacity)?;
            let submission = self.owner.admit_locked(
                &mut state,
                context,
                bytes,
                deadline,
                Instant::now(),
                &mut scope,
            )?;
            let key = submission.observer.key;
            Core::entry(&mut state, key)
                .expect("newly admitted original entry")
                .recipe = Some(Recipe::new(route.key, charge));
            let registry = state.coordinator.as_mut().unwrap();
            registry.used = used;
            registry.roots[route.key.index].as_mut().unwrap().jobs = jobs;
            Ok(CoordinatedSubmission {
                submission,
                job: Job::new(self.owner.core.clone(), key),
            })
        })();
        result.map_err(|reason| BoundRefusal {
            reason,
            scope: scope.take().expect("unadmitted scope retained"),
        })
    }
    pub(in crate::runtime) fn close(&self, route: &Route) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.owner.core, &route.core) {
            return Err(Error::Stale);
        }
        let mut state = self.owner.core.lock()?;
        let registry = state.coordinator.as_mut().ok_or(Error::Unsupported)?;
        registry.route(route.key)?;
        registry.roots[route.key.index].as_mut().unwrap().closing = true;
        Ok(())
    }
    pub(in crate::runtime) fn remove(&self, route: &Route) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.owner.core, &route.core) {
            return Err(Error::Stale);
        }
        let retired = {
            let mut state = self.owner.core.lock()?;
            let registry = state.coordinator.as_mut().ok_or(Error::Unsupported)?;
            if registry.route(route.key)?.jobs != 0 {
                return Err(Error::Pending);
            }
            registry.roots[route.key.index]
                .as_mut()
                .unwrap()
                .scope
                .take()
        };
        drop(retired); // Last scope/lease/permit never drops under the Core lock.
        Ok(())
    }
    #[cfg(test)]
    pub(in crate::runtime) fn fixture_new(profile: Profile) -> Result<Self, Error> {
        let coordinator = Self::new(profile)?;
        coordinator
            .owner
            .core
            .lock()?
            .coordinator
            .as_mut()
            .unwrap()
            .fixture_supported = true;
        Ok(coordinator)
    }
    #[cfg(test)]
    pub(in crate::runtime) fn fixture_register(
        &self,
        root: u128,
        incarnation: u64,
        scope: OriginalInputScope,
    ) -> Result<Route, BoundRefusal> {
        let mut scope = Some(scope);
        let result = (|| {
            if !self.scopes.owns(scope.as_ref().unwrap()) {
                return Err(Error::Stale);
            }
            let mut state = self.owner.core.lock()?;
            let registry = state.coordinator.as_mut().unwrap();
            if !registry.fixture_supported {
                return Err(Error::Unsupported);
            }
            if registry.roots.iter().flatten().any(|r| r.root == root) {
                return Err(Error::Duplicate);
            }
            let index = registry
                .roots
                .iter()
                .position(Option::is_none)
                .ok_or(Error::Capacity)?;
            let serial = registry.serial.checked_add(1).ok_or(Error::Capacity)?;
            registry.serial = serial;
            let key = RouteKey { index, serial };
            registry.roots[index] = Some(Registration {
                key,
                root,
                incarnation,
                scope: scope.take(),
                closing: false,
                jobs: 0,
            });
            Ok(Route {
                core: self.owner.core.clone(),
                key,
            })
        })();
        result.map_err(|reason| BoundRefusal {
            reason,
            scope: scope.take().unwrap(),
        })
    }
    #[cfg(test)]
    pub(in crate::runtime) fn fixture_expire(&self, context: Context) {
        self.owner
            .core
            .lock()
            .unwrap()
            .slots
            .iter_mut()
            .flatten()
            .find(|e| e.context == context)
            .unwrap()
            .deadline = Instant::now();
    }
}

#[cfg(test)]
#[path = "coordinator/tests.rs"]
pub(crate) mod tests;
