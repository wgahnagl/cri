use crate::{
    criapi::{RunPodSandboxRequest, RunPodSandboxResponse},
    runtime_service::MyRuntime,
    sandbox::{pinned::PinnedSandbox, SandboxBuilder, SandboxDataBuilder},
};
use log::{debug, info};
use tonic::{Request, Response, Status};

impl MyRuntime {
    pub async fn handle_run_pod_sandbox(
        &self,
        request: Request<RunPodSandboxRequest>,
    ) -> Result<Response<RunPodSandboxResponse>, Status> {
        // Take the pod sandbox config
        let config = request
            .into_inner()
            .config
            .take()
            .ok_or_else(|| Status::invalid_argument("no pod sandbox config provided"))?;

        // Verify that the metadata exists
        let metadata = config
            .metadata
            .ok_or_else(|| Status::invalid_argument("no pod sandbox metadata provided"))?;

        // Build a new sandbox from it
        let mut sandbox = SandboxBuilder::<PinnedSandbox>::default()
            .data(
                SandboxDataBuilder::default()
                    .id(metadata.uid)
                    .name(metadata.name)
                    .namespace(metadata.namespace)
                    .attempt(metadata.attempt)
                    .build()
                    .map_err(|e| {
                        Status::internal(format!("build sandbox data from metadata: {}", e))
                    })?,
            )
            .build()
            .map_err(|e| Status::internal(format!("build sandbox from config: {}", e)))?;

        debug!("Created pod sandbox {:?}", sandbox);

        // Run the sandbox
        sandbox
            .run()
            .map_err(|e| Status::internal(format!("run pod sandbox: {}", e)))?;
        info!("Started pod sandbox {}", sandbox);

        // Build and return the response
        let reply = RunPodSandboxResponse {
            pod_sandbox_id: sandbox.id().into(),
        };
        Ok(Response::new(reply))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::criapi::{
        runtime_service_server::RuntimeService, PodSandboxConfig, PodSandboxMetadata,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn run_pod_sandbox_success() {
        let sut = MyRuntime::default();
        let test_id = "123";
        let request = RunPodSandboxRequest {
            config: Some(PodSandboxConfig {
                metadata: Some(PodSandboxMetadata {
                    name: "".into(),
                    uid: test_id.into(),
                    namespace: "".into(),
                    attempt: 0,
                }),
                hostname: "".into(),
                log_directory: "".into(),
                dns_config: None,
                port_mappings: vec![],
                labels: HashMap::new(),
                annotations: HashMap::new(),
                linux: None,
            }),
            runtime_handler: "".into(),
        };
        let response = sut.run_pod_sandbox(Request::new(request)).await.unwrap();
        assert_eq!(response.get_ref().pod_sandbox_id, test_id);
    }

    #[tokio::test]
    async fn run_pod_sandbox_fail_no_config() {
        let sut = MyRuntime::default();
        let request = RunPodSandboxRequest {
            config: None,
            runtime_handler: "".into(),
        };
        let response = sut.run_pod_sandbox(Request::new(request)).await;
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn run_pod_sandbox_fail_no_config_metadata() {
        let sut = MyRuntime::default();
        let request = RunPodSandboxRequest {
            config: Some(PodSandboxConfig {
                metadata: None,
                hostname: "".into(),
                log_directory: "".into(),
                dns_config: None,
                port_mappings: vec![],
                labels: HashMap::new(),
                annotations: HashMap::new(),
                linux: None,
            }),
            runtime_handler: "".into(),
        };
        let response = sut.run_pod_sandbox(Request::new(request)).await;
        assert!(response.is_err());
    }
}
