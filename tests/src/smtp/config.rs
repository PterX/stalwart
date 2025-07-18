/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::{fs, net::IpAddr, path::PathBuf, sync::Arc, time::Duration};

use common::{
    Server,
    config::{
        server::{Listener, Listeners, ServerProtocol, TcpListener},
        smtp::*,
    },
    expr::{functions::ResolveVariable, if_block::*, tokenizer::TokenMap, *},
};

use compact_str::ToCompactString;
use throttle::parse_queue_rate_limiter;
use tokio::net::TcpSocket;

use utils::config::{Config, Rate};

use super::add_test_certs;

struct TestEnvelope {
    pub local_ip: IpAddr,
    pub remote_ip: IpAddr,
    pub sender_domain: String,
    pub sender: String,
    pub rcpt_domain: String,
    pub rcpt: String,
    pub helo_domain: String,
    pub authenticated_as: String,
    pub mx: String,
    pub listener_id: String,
    pub priority: i16,
}

#[test]
fn parse_if_blocks() {
    let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file.push("resources");
    file.push("smtp");
    file.push("config");
    file.push("if-blocks.toml");

    let mut config = Config::new(fs::read_to_string(file).unwrap()).unwrap();

    // Create context and add some conditions

    let token_map = TokenMap::default().with_variables(&[
        V_RECIPIENT,
        V_RECIPIENT_DOMAIN,
        V_SENDER,
        V_SENDER_DOMAIN,
        V_AUTHENTICATED_AS,
        V_LISTENER,
        V_REMOTE_IP,
        V_LOCAL_IP,
        V_PRIORITY,
    ]);

    assert_eq!(
        IfBlock::try_parse(&mut config, "durations", &token_map).unwrap(),
        IfBlock {
            key: "durations".into(),
            if_then: vec![
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_SENDER),
                            ExpressionItem::Constant(Constant::String("jdoe".into())),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq)
                        ]
                    },
                    then: Expression {
                        items: vec![ExpressionItem::Constant(Constant::Integer(432000000))]
                    }
                },
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_PRIORITY),
                            ExpressionItem::Constant(Constant::Integer(1)),
                            ExpressionItem::UnaryOperator(UnaryOperator::Minus),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq),
                            ExpressionItem::JmpIf { val: true, pos: 4 },
                            ExpressionItem::Variable(V_RECIPIENT),
                            ExpressionItem::Constant(Constant::String("jane".into())),
                            ExpressionItem::Function {
                                id: 29,
                                num_args: 2
                            },
                            ExpressionItem::BinaryOperator(BinaryOperator::Or)
                        ]
                    },
                    then: Expression {
                        items: vec![ExpressionItem::Constant(Constant::Integer(3600000))]
                    }
                }
            ],
            default: Expression {
                items: vec![ExpressionItem::Constant(Constant::Integer(0))]
            }
        }
    );

    assert_eq!(
        IfBlock::try_parse(&mut config, "string-list", &token_map).unwrap(),
        IfBlock {
            key: "string-list".into(),
            if_then: vec![
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_SENDER),
                            ExpressionItem::Constant(Constant::String("jdoe".into())),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq)
                        ]
                    },
                    then: Expression {
                        items: vec![
                            ExpressionItem::Constant(Constant::String("From".into())),
                            ExpressionItem::Constant(Constant::String("To".into())),
                            ExpressionItem::Constant(Constant::String("Date".into())),
                            ExpressionItem::ArrayBuild(3)
                        ]
                    }
                },
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_PRIORITY),
                            ExpressionItem::Constant(Constant::Integer(1)),
                            ExpressionItem::UnaryOperator(UnaryOperator::Minus),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq),
                            ExpressionItem::JmpIf { val: true, pos: 4 },
                            ExpressionItem::Variable(V_RECIPIENT),
                            ExpressionItem::Constant(Constant::String("jane".into())),
                            ExpressionItem::Function {
                                id: 29,
                                num_args: 2
                            },
                            ExpressionItem::BinaryOperator(BinaryOperator::Or)
                        ]
                    },
                    then: Expression {
                        items: vec![ExpressionItem::Constant(Constant::String(
                            "Other-ID".into()
                        ))]
                    }
                }
            ],
            default: Expression {
                items: vec![ExpressionItem::ArrayBuild(0)]
            }
        }
    );

    assert_eq!(
        IfBlock::try_parse(&mut config, "string-list-bis", &token_map).unwrap(),
        IfBlock {
            key: "string-list-bis".into(),
            if_then: vec![
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_SENDER),
                            ExpressionItem::Constant(Constant::String("jdoe".into())),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq)
                        ]
                    },
                    then: Expression {
                        items: vec![
                            ExpressionItem::Constant(Constant::String("From".into())),
                            ExpressionItem::Constant(Constant::String("To".into())),
                            ExpressionItem::Constant(Constant::String("Date".into())),
                            ExpressionItem::ArrayBuild(3)
                        ]
                    }
                },
                IfThen {
                    expr: Expression {
                        items: vec![
                            ExpressionItem::Variable(V_PRIORITY),
                            ExpressionItem::Constant(Constant::Integer(1)),
                            ExpressionItem::UnaryOperator(UnaryOperator::Minus),
                            ExpressionItem::BinaryOperator(BinaryOperator::Eq),
                            ExpressionItem::JmpIf { val: true, pos: 4 },
                            ExpressionItem::Variable(V_RECIPIENT),
                            ExpressionItem::Constant(Constant::String("jane".into())),
                            ExpressionItem::Function {
                                id: 29,
                                num_args: 2
                            },
                            ExpressionItem::BinaryOperator(BinaryOperator::Or)
                        ]
                    },
                    then: Expression {
                        items: vec![ExpressionItem::ArrayBuild(0)]
                    }
                }
            ],
            default: Expression {
                items: vec![
                    ExpressionItem::Constant(Constant::String("ID-Bis".into())),
                    ExpressionItem::ArrayBuild(1)
                ]
            }
        }
    );

    assert_eq!(
        IfBlock::try_parse(&mut config, "single-value", &token_map).unwrap(),
        IfBlock {
            key: "single-value".into(),
            if_then: vec![],
            default: Expression {
                items: vec![ExpressionItem::Constant(Constant::String(
                    "hello world".into()
                ))]
            }
        }
    );

    for bad_rule in [
        "bad-if-without-then",
        "bad-if-without-else",
        "bad-multiple-else",
    ] {
        if let Some(value) = IfBlock::try_parse(&mut config, bad_rule, &token_map) {
            panic!("Condition {bad_rule:?} had unexpected result {value:?}");
        }
    }
}

#[test]
fn parse_throttles() {
    let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file.push("resources");
    file.push("smtp");
    file.push("config");
    file.push("throttle.toml");

    let mut config = Config::new(fs::read_to_string(file).unwrap()).unwrap();
    let throttle = parse_queue_rate_limiter(
        &mut config,
        "throttle",
        &TokenMap::default().with_variables(&[
            V_RECIPIENT,
            V_RECIPIENT_DOMAIN,
            V_SENDER,
            V_SENDER_DOMAIN,
            V_AUTHENTICATED_AS,
            V_LISTENER,
            V_REMOTE_IP,
            V_LOCAL_IP,
            V_PRIORITY,
        ]),
        u16::MAX,
    );

    assert_eq!(
        throttle,
        vec![
            QueueRateLimiter {
                id: "0000".into(),
                expr: Expression {
                    items: vec![
                        ExpressionItem::Variable(8),
                        ExpressionItem::Constant(Constant::String("127.0.0.1".into())),
                        ExpressionItem::BinaryOperator(BinaryOperator::Eq)
                    ]
                },
                keys: THROTTLE_REMOTE_IP | THROTTLE_AUTH_AS,
                rate: Rate {
                    requests: 50,
                    period: Duration::from_secs(30)
                }
            },
            QueueRateLimiter {
                id: "0001".into(),
                expr: Expression::default(),
                keys: THROTTLE_SENDER_DOMAIN,
                rate: Rate {
                    requests: 50,
                    period: Duration::from_secs(30)
                }
            }
        ]
    );
}

#[test]
fn parse_servers() {
    let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file.push("resources");
    file.push("smtp");
    file.push("config");
    file.push("servers.toml");

    let toml = add_test_certs(&fs::read_to_string(file).unwrap());

    // Parse servers
    let mut config = Config::new(toml).unwrap();
    let servers = Listeners::parse(&mut config).servers;
    let id_generator = Arc::new(utils::snowflake::SnowflakeIdGenerator::new());
    let expected_servers = vec![
        Listener {
            id: "smtp".into(),
            protocol: ServerProtocol::Smtp,
            listeners: vec![TcpListener {
                socket: TcpSocket::new_v4().unwrap(),
                addr: "127.0.0.1:9925".parse().unwrap(),
                ttl: 3600.into(),
                backlog: 1024.into(),
                linger: None,
                nodelay: true,
            }],
            max_connections: 8192,
            proxy_networks: vec![],
            span_id_gen: id_generator.clone(),
        },
        Listener {
            id: "smtps".into(),
            protocol: ServerProtocol::Smtp,
            listeners: vec![
                TcpListener {
                    socket: TcpSocket::new_v4().unwrap(),
                    addr: "127.0.0.1:9465".parse().unwrap(),
                    ttl: 4096.into(),
                    backlog: 1024.into(),
                    linger: None,
                    nodelay: true,
                },
                TcpListener {
                    socket: TcpSocket::new_v4().unwrap(),
                    addr: "127.0.0.1:9466".parse().unwrap(),
                    ttl: 4096.into(),
                    backlog: 1024.into(),
                    linger: None,
                    nodelay: true,
                },
            ],
            max_connections: 1024,
            proxy_networks: vec![],
            span_id_gen: id_generator.clone(),
        },
        Listener {
            id: "submission".into(),
            protocol: ServerProtocol::Smtp,
            listeners: vec![TcpListener {
                socket: TcpSocket::new_v4().unwrap(),
                addr: "127.0.0.1:9991".parse().unwrap(),
                ttl: 3600.into(),
                backlog: 2048.into(),
                linger: None,
                nodelay: true,
            }],
            max_connections: 8192,
            proxy_networks: vec![],
            span_id_gen: id_generator.clone(),
        },
    ];

    for (server, expected_server) in servers.into_iter().zip(expected_servers) {
        assert_eq!(
            server.id, expected_server.id,
            "failed for {}",
            expected_server.id
        );
        assert_eq!(
            server.protocol, expected_server.protocol,
            "failed for {}",
            expected_server.id
        );
        for (listener, expected_listener) in
            server.listeners.into_iter().zip(expected_server.listeners)
        {
            assert_eq!(
                listener.addr, expected_listener.addr,
                "failed for {}",
                expected_server.id
            );
            assert_eq!(
                listener.ttl, expected_listener.ttl,
                "failed for {}",
                expected_server.id
            );
            assert_eq!(
                listener.backlog, expected_listener.backlog,
                "failed for {}",
                expected_server.id
            );
        }
    }
}

#[tokio::test]
async fn eval_if() {
    let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file.push("resources");
    file.push("smtp");
    file.push("config");
    file.push("rules-eval.toml");

    let mut config = Config::new(fs::read_to_string(file).unwrap()).unwrap();
    let envelope = TestEnvelope::from_config(&mut config);
    let token_map = TokenMap::default().with_variables(&[
        V_RECIPIENT,
        V_RECIPIENT_DOMAIN,
        V_SENDER,
        V_SENDER_DOMAIN,
        V_AUTHENTICATED_AS,
        V_LISTENER,
        V_REMOTE_IP,
        V_LOCAL_IP,
        V_PRIORITY,
        V_MX,
    ]);
    let core = Server::default();

    for (key, _) in config.keys.clone() {
        if !key.starts_with("rule.") {
            continue;
        }

        //println!("============= Testing {:?} ==================", key);
        let (_, expected_result) = key.rsplit_once('-').unwrap();
        assert_eq!(
            core.eval_if::<Variable, _>(
                &IfBlock {
                    key: key.to_string(),
                    if_then: vec![IfThen {
                        expr: Expression::try_parse(&mut config, key.as_str(), &token_map).unwrap(),
                        then: Expression::from(true),
                    }],
                    default: Expression::from(false),
                },
                &envelope,
                0
            )
            .await
            .unwrap()
            .to_bool(),
            expected_result.parse::<bool>().unwrap(),
            "failed for {key:?}"
        );
    }
}

#[tokio::test]
async fn eval_dynvalue() {
    let mut file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file.push("resources");
    file.push("smtp");
    file.push("config");
    file.push("rules-dynvalue.toml");

    let mut config = Config::new(fs::read_to_string(file).unwrap()).unwrap();
    let envelope = TestEnvelope::from_config(&mut config);
    let token_map = TokenMap::default().with_variables(&[
        V_RECIPIENT,
        V_RECIPIENT_DOMAIN,
        V_SENDER,
        V_SENDER_DOMAIN,
        V_AUTHENTICATED_AS,
        V_LISTENER,
        V_REMOTE_IP,
        V_LOCAL_IP,
        V_PRIORITY,
        V_MX,
    ]);
    let core = Server::default();

    for test_name in config.sub_keys("eval", "") {
        //println!("============= Testing {:?} ==================", key);
        let if_block = IfBlock::try_parse(
            &mut config,
            ("eval", test_name.as_str(), "test"),
            &token_map,
        )
        .unwrap();
        let expected = config
            .property_require::<Option<String>>(("eval", test_name.as_str(), "expect"))
            .unwrap_or_else(|| panic!("Missing expect for test {test_name:?}"));

        assert_eq!(
            core.eval_if::<String, _>(&if_block, &envelope, 0).await,
            expected,
            "failed for test {test_name:?}"
        );
    }
}

impl ResolveVariable for TestEnvelope {
    fn resolve_variable(&self, variable: u32) -> Variable<'_> {
        match variable {
            V_RECIPIENT => self.rcpt.as_str().into(),
            V_RECIPIENT_DOMAIN => self.rcpt_domain.as_str().into(),
            V_SENDER => self.sender.as_str().into(),
            V_SENDER_DOMAIN => self.sender_domain.as_str().into(),
            V_AUTHENTICATED_AS => self.authenticated_as.as_str().into(),
            V_LISTENER => self.listener_id.to_compact_string().into(),
            V_REMOTE_IP => self.remote_ip.to_compact_string().into(),
            V_LOCAL_IP => self.local_ip.to_compact_string().into(),
            V_PRIORITY => self.priority.to_compact_string().into(),
            V_MX => self.mx.as_str().into(),
            V_HELO_DOMAIN => self.helo_domain.as_str().into(),
            _ => Default::default(),
        }
    }

    fn resolve_global(&self, _: &str) -> Variable<'_> {
        Variable::Integer(0)
    }
}

impl TestEnvelope {
    pub fn from_config(config: &mut Config) -> Self {
        Self {
            local_ip: config.property_require("envelope.local-ip").unwrap(),
            remote_ip: config.property_require("envelope.remote-ip").unwrap(),
            sender_domain: config.property_require("envelope.sender-domain").unwrap(),
            sender: config.property_require("envelope.sender").unwrap(),
            rcpt_domain: config.property_require("envelope.rcpt-domain").unwrap(),
            rcpt: config.property_require("envelope.rcpt").unwrap(),
            authenticated_as: config
                .property_require("envelope.authenticated-as")
                .unwrap(),
            mx: config.property_require("envelope.mx").unwrap(),
            listener_id: config.property_require("envelope.listener").unwrap(),
            priority: config.property_require("envelope.priority").unwrap(),
            helo_domain: config.property_require("envelope.helo-domain").unwrap(),
        }
    }
}
