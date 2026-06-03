//! Execution simulation boundary.
//!
//! This crate will simulate trade execution from candidate routes.

#![forbid(unsafe_code)]

pub mod simulator {
    //! Execution simulation and outcome modeling responsibilities.

    use common::Result;
    use routing::Router;

    /// Placeholder execution simulator handle.
    #[derive(Clone, Copy, Debug)]
    pub struct ExecutionSimulator {
        router: Router,
    }

    impl ExecutionSimulator {
        /// Creates a placeholder execution simulator.
        #[must_use]
        pub const fn new(router: Router) -> Self {
            Self { router }
        }

        /// Returns the router boundary used by execution.
        #[must_use]
        pub const fn router(&self) -> Router {
            self.router
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use simulator::ExecutionSimulator;

#[cfg(test)]
mod tests {
    use super::ExecutionSimulator;
    use routing::Router;

    #[test]
    fn execution_simulator_placeholder_is_ready() {
        let simulator = ExecutionSimulator::new(Router::default());

        assert!(simulator.ready().is_ok());
        assert!(simulator.router().graph().validate().is_ok());
    }
}
