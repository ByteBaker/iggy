// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use assert_cmd::Command;
use iggy::prelude::defaults::{DEFAULT_ROOT_PASSWORD, DEFAULT_ROOT_USER_ID, DEFAULT_ROOT_USERNAME};
use iggy::prelude::{
    Client, IggyError, SystemClient, TcpClient, TcpClientConfig, TcpClientReconnectionConfig,
    UserClient,
};
use integration::harness::{TestHarnessBuilder, TestServerConfig, TlsConfig};
use predicates::str::contains;
use serial_test::parallel;

use super::common::TestIggyContext;
use crate::cli::common::clear_session_entry;

#[tokio::test]
#[parallel]
async fn should_match_sdk_tls_behavior_with_cli_flags_and_contexts() {
    let mut harness = TestHarnessBuilder::default()
        .cluster_nodes(1)
        .server(
            TestServerConfig::builder()
                .tls(TlsConfig::generated())
                .build(),
        )
        .build()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(60), harness.start())
        .await
        .expect("TLS server startup timed out")
        .unwrap();
    clear_session_entry(harness.server().tcp_addr().unwrap());
    let cli_home = tempfile::tempdir().unwrap();
    let context = TestIggyContext::new(None, None);
    let context_home = context.get_iggy_home();
    let address = harness.server().raw_tcp_addr().unwrap();
    let ca_file = harness.server().tls_ca_cert_path().unwrap();
    let missing_ca = cli_home.path().join("missing-ca.pem");
    let wrong_ca = cli_home.path().join("wrong-ca.pem");
    let unrelated_cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    std::fs::write(&wrong_ca, unrelated_cert.cert.pem()).unwrap();

    for (name, ca, domain, error) in [
        ("trusted-ca", Some(&ca_file), "localhost", None),
        (
            "no-ca",
            None,
            "localhost",
            Some(IggyError::CannotEstablishConnection),
        ),
        (
            "missing-ca",
            Some(&missing_ca),
            "localhost",
            Some(IggyError::InvalidTlsCertificatePath),
        ),
        (
            "wrong-ca",
            Some(&wrong_ca),
            "localhost",
            Some(IggyError::CannotEstablishConnection),
        ),
        (
            "wrong-hostname",
            Some(&ca_file),
            "mismatch.invalid",
            Some(IggyError::CannotEstablishConnection),
        ),
    ] {
        let client = TcpClient::create(Arc::new(TcpClientConfig {
            server_address: address.clone(),
            tls_enabled: true,
            tls_domain: domain.to_string(),
            tls_ca_file: ca.map(|path| path.to_str().unwrap().to_string()),
            reconnection: TcpClientReconnectionConfig {
                max_retries: Some(0),
                ..Default::default()
            },
            ..Default::default()
        }))
        .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(15), async {
            client.connect().await?;
            client
                .login_user(DEFAULT_ROOT_USERNAME, DEFAULT_ROOT_PASSWORD)
                .await?;
            client.get_me().await
        })
        .await
        .expect("SDK TLS request timed out");
        client.shutdown().await.unwrap();
        if let Some(error) = &error {
            assert_eq!(result.unwrap_err(), *error, "SDK: {name}");
        } else {
            assert_eq!(result.unwrap().user_id, Some(DEFAULT_ROOT_USER_ID));
        }

        let mut flags_command = cli_command(cli_home.path());
        flags_command.arg("--tcp-tls-enabled");
        let mut create_command = cli_command(&context_home);
        create_command.args([
            "context",
            "create",
            name,
            "--transport",
            "tcp",
            "--tcp-tls-enabled",
            "true",
        ]);
        let tls_args = ["--tcp-server-address", &address, "--tcp-tls-domain", domain];
        for command in [&mut flags_command, &mut create_command] {
            command.args(tls_args);
            if let Some(ca) = ca {
                command.arg("--tcp-tls-ca-file").arg(ca);
            }
        }
        create_command.assert().success();
        let contexts = context.read_saved_contexts().await.unwrap();
        let saved = &contexts[name];
        assert_eq!(saved.iggy.tcp_tls_enabled, Some(true));
        assert_eq!(saved.iggy.tcp_tls_domain.as_deref(), Some(domain));
        assert_eq!(
            saved.iggy.tcp_tls_ca_file.as_deref(),
            ca.map(|path| path.to_str().unwrap())
        );
        cli_command(&context_home)
            .args(["context", "use", name])
            .assert()
            .success();

        for (source, mut command) in [
            ("flags", flags_command),
            ("context", cli_command(&context_home)),
        ] {
            let assertion = command.arg("me").assert();
            if let Some(error) = &error {
                // A timeout must not count as a TLS rejection.
                assertion
                    .code(1)
                    .stderr(contains(error.to_string()))
                    .stdout("");
            } else {
                assertion.success().stdout(contains("User ID"));
            }
            println!("{name}: SDK and CLI {source} agree");
        }

        if error.is_none() {
            cli_command(&context_home)
                .arg("--tcp-tls-ca-file")
                .arg(&missing_ca)
                .arg("me")
                .assert()
                .code(1)
                .stderr(contains(IggyError::InvalidTlsCertificatePath.to_string()));
            cli_command(&context_home)
                .args(["--tcp-tls-domain", "mismatch.invalid", "me"])
                .assert()
                .code(1)
                .stderr(contains(IggyError::CannotEstablishConnection.to_string()));
        }
        let current = context.read_saved_contexts().await.unwrap();
        assert_eq!(
            serde_json::to_value(&current[name]).unwrap(),
            serde_json::to_value(saved).unwrap()
        );
    }
}

fn cli_command(home: &Path) -> Command {
    #[allow(deprecated)]
    let mut command = Command::cargo_bin("iggy").unwrap();
    command
        .env("IGGY_HOME", home)
        .env("COLUMNS", "200")
        .args([
            "--username",
            DEFAULT_ROOT_USERNAME,
            "--password",
            DEFAULT_ROOT_PASSWORD,
            "--tcp-reconnection-max-retries",
            "0",
        ])
        .timeout(Duration::from_secs(15));
    command
}
