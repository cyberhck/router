use crate::context::{OPERATION_KIND, OPERATION_NAME};
use crate::plugin::serde::deserialize_json_query;
use crate::plugins::telemetry::config::AttributeValue;
use crate::plugins::telemetry::config_new::GetAttribute;
use crate::services::{router, subgraph, supergraph};
use access_json::JSONQuery;
use opentelemetry_api::baggage::BaggageExt;
use opentelemetry_api::trace::TraceContextExt;
use opentelemetry_api::Context;
use schemars::JsonSchema;
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;
use serde_json_bytes::ByteString;
use sha2::Digest;

#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum TraceIdFormat {
    /// Open Telemetry trace ID, a hex string.
    OpenTelemetry,
    /// Datadog trace ID, a u64.
    Datadog,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum OperationName {
    /// The raw operation name.
    String,
    /// A hash of the operation name.
    Hash,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum Query {
    /// The raw query kind.
    String,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum OperationKind {
    /// The raw operation kind.
    String,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum ResponseStatus {
    /// The http status code.
    Code,
    /// The http status reason.
    Reason,
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[serde(deny_unknown_fields, untagged)]
pub(crate) enum RouterSelector {
    /// A header from the request
    RequestHeader {
        /// The name of the request header.
        request_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    /// A header from the response
    ResponseHeader {
        /// The name of the request header.
        response_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    /// A header from the response
    ResponseStatus {
        /// The http response status code.
        response_status: ResponseStatus,
    },
    /// The trace ID of the request.
    TraceId {
        /// The format of the trace ID.
        trace_id: TraceIdFormat,
    },
    /// A value from context.
    ResponseContext {
        /// The response context key.
        response_context: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    /// A value from baggage.
    Baggage {
        /// The name of the baggage item.
        baggage: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    /// A value from an environment variable.
    Env {
        /// The name of the environment variable
        env: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields, untagged)]
pub(crate) enum SupergraphSelector {
    OperationName {
        /// The operation name from the query.
        operation_name: OperationName,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    OperationKind {
        /// The operation kind from the query (query|mutation|subscription).
        operation_kind: OperationKind,
    },
    Query {
        /// The graphql query.
        query: Query,
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    QueryVariable {
        /// The name of a graphql query variable.
        query_variable: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    RequestHeader {
        /// The name of the request header.
        request_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    ResponseHeader {
        /// The name of the response header.
        response_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    RequestContext {
        /// The request context key.
        request_context: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    ResponseContext {
        /// The response context key.
        response_context: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    Baggage {
        /// The name of the baggage item.
        baggage: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    Env {
        /// The name of the environment variable
        env: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
}

#[allow(dead_code)]
#[derive(Deserialize, JsonSchema, Clone, Debug)]
#[serde(deny_unknown_fields, rename_all = "snake_case", untagged)]
pub(crate) enum SubgraphSelector {
    SubgraphOperationName {
        /// The operation name from the subgraph query.
        subgraph_operation_name: OperationName,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    SubgraphOperationKind {
        /// The kind of the subgraph operation (query|mutation|subscription).
        subgraph_operation_kind: OperationKind,
    },
    SubgraphQuery {
        /// The graphql query to the subgraph.
        subgraph_query: Query,
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    SubgraphQueryVariable {
        /// The name of a subgraph query variable.
        subgraph_query_variable: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    SubgraphResponseBody {
        /// The subgraph response body json path.
        #[schemars(with = "String")]
        #[serde(deserialize_with = "deserialize_json_query")]
        subgraph_response_body: JSONQuery,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    SubgraphRequestHeader {
        /// The name of a subgraph request header.
        subgraph_request_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    SubgraphResponseHeader {
        /// The name of a subgraph response header.
        subgraph_response_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    SubgraphResponseStatus {
        /// The subgraph http response status code.
        subgraph_response_status: ResponseStatus,
    },
    SupergraphOperationName {
        /// The supergraph query operation name.
        supergraph_operation_name: OperationName,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    SupergraphOperationKind {
        /// The supergraph query operation kind (query|mutation|subscription).
        supergraph_operation_kind: OperationKind,
    },
    SupergraphQuery {
        /// The supergraph query to the subgraph.
        supergraph_query: Query,
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
    SupergraphQueryVariable {
        /// The supergraph query variable name.
        supergraph_query_variable: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    SupergraphRequestHeader {
        /// The supergraph request header name.
        supergraph_request_header: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    RequestContext {
        /// The request context key.
        request_context: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    ResponseContext {
        /// The response context key.
        response_context: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    Baggage {
        /// The name of the baggage item.
        baggage: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<AttributeValue>,
    },
    Env {
        /// The name of the environment variable
        env: String,
        #[serde(skip)]
        /// Optional redaction pattern.
        redact: Option<String>,
        /// Optional default value.
        default: Option<String>,
    },
}

impl GetAttribute<router::Request, router::Response> for RouterSelector {
    fn on_request(&self, request: &router::Request) -> Option<AttributeValue> {
        match self {
            RouterSelector::RequestHeader {
                request_header,
                default,
                ..
            } => request
                .router_request
                .headers()
                .get(request_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            RouterSelector::Env { env, default, .. } => std::env::var(env)
                .ok()
                .map(AttributeValue::String)
                .or_else(|| default.clone().map(AttributeValue::String)),
            RouterSelector::TraceId {
                trace_id: trace_id_format,
            } => {
                if Context::current().span().span_context().is_valid() {
                    let id = Context::current().span().span_context().trace_id();
                    match trace_id_format {
                        TraceIdFormat::OpenTelemetry => AttributeValue::String(id.to_string()),
                        TraceIdFormat::Datadog => {
                            AttributeValue::U128(u128::from_be_bytes(id.to_bytes()))
                        }
                    }
                    .into()
                } else {
                    None
                }
            }
            RouterSelector::Baggage {
                baggage: baggage_name,
                default,
                ..
            } => {
                let context = Context::current();
                let baggage = context.baggage();
                match baggage.get(baggage_name.to_string()) {
                    Some(baggage) => AttributeValue::from(baggage.clone()).into(),
                    None => default.clone(),
                }
            }
            // Related to Response
            _ => None,
        }
    }

    fn on_response(&self, response: &router::Response) -> Option<AttributeValue> {
        match self {
            RouterSelector::ResponseHeader {
                response_header,
                default,
                ..
            } => response
                .response
                .headers()
                .get(response_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            RouterSelector::ResponseStatus { response_status } => match response_status {
                ResponseStatus::Code => Some(AttributeValue::I64(
                    response.response.status().as_u16() as i64,
                )),
                ResponseStatus::Reason => response
                    .response
                    .status()
                    .canonical_reason()
                    .map(|reason| AttributeValue::String(reason.to_string())),
            },
            RouterSelector::ResponseContext {
                response_context,
                default,
                ..
            } => response
                .context
                .get(response_context)
                .ok()
                .flatten()
                .or_else(|| default.clone()),
            RouterSelector::Baggage {
                baggage: baggage_name,
                default,
                ..
            } => {
                let span_context = Context::current();
                // I must clone the key because the otel API is bad
                let baggage = span_context.baggage().get(baggage_name.clone()).cloned();
                match baggage {
                    Some(baggage) => AttributeValue::from(baggage).into(),
                    None => default.clone(),
                }
            }
            _ => None,
        }
    }
}

impl GetAttribute<supergraph::Request, supergraph::Response> for SupergraphSelector {
    fn on_request(&self, request: &supergraph::Request) -> Option<AttributeValue> {
        match self {
            SupergraphSelector::OperationName {
                operation_name,
                default,
                ..
            } => {
                let op_name = request.context.get(OPERATION_NAME).ok().flatten();
                match operation_name {
                    OperationName::String => op_name.or_else(|| default.clone()),
                    OperationName::Hash => op_name.or_else(|| default.clone()).map(|op_name| {
                        let mut hasher = sha2::Sha256::new();
                        hasher.update(op_name.as_bytes());
                        let result = hasher.finalize();
                        hex::encode(result)
                    }),
                }
                .map(AttributeValue::String)
            }
            SupergraphSelector::OperationKind { .. } => {
                request.context.get(OPERATION_KIND).ok().flatten()
            }
            SupergraphSelector::Query { default, .. } => request
                .supergraph_request
                .body()
                .query
                .clone()
                .or_else(|| default.clone())
                .map(AttributeValue::String),
            SupergraphSelector::QueryVariable {
                query_variable,
                default,
                ..
            } => request
                .supergraph_request
                .body()
                .variables
                .get(&ByteString::from(query_variable.as_str()))
                .and_then(|v| serde_json::to_string(v).ok())
                .map(AttributeValue::String)
                .or_else(|| default.clone()),
            SupergraphSelector::RequestHeader {
                request_header,
                default,
                ..
            } => request
                .supergraph_request
                .headers()
                .get(request_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            SupergraphSelector::RequestContext {
                request_context,
                default,
                ..
            } => request
                .context
                .get(request_context)
                .ok()
                .flatten()
                .or_else(|| default.clone()),
            SupergraphSelector::Baggage {
                baggage: baggage_name,
                default,
                ..
            } => {
                let span_context = Context::current();
                // I must clone the key because the otel API is bad
                let baggage = span_context.baggage().get(baggage_name.clone()).cloned();
                match baggage {
                    Some(baggage) => AttributeValue::from(baggage.clone()).into(),
                    None => default.clone(),
                }
            }
            SupergraphSelector::Env { env, default, .. } => std::env::var(env)
                .ok()
                .map(AttributeValue::String)
                .or_else(|| default.clone().map(AttributeValue::String)),
            // For response
            _ => None,
        }
    }

    fn on_response(&self, response: &supergraph::Response) -> Option<AttributeValue> {
        match self {
            SupergraphSelector::ResponseHeader {
                response_header,
                default,
                ..
            } => response
                .response
                .headers()
                .get(response_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            SupergraphSelector::ResponseContext {
                response_context,
                default,
                ..
            } => response
                .context
                .get(response_context)
                .ok()
                .flatten()
                .or_else(|| default.clone()),
            // For request
            _ => None,
        }
    }
}

impl GetAttribute<subgraph::Request, subgraph::Response> for SubgraphSelector {
    fn on_request(&self, request: &subgraph::Request) -> Option<AttributeValue> {
        match self {
            SubgraphSelector::SubgraphOperationName {
                subgraph_operation_name,
                default,
                ..
            } => {
                let op_name = request.subgraph_request.body().operation_name.clone();
                match subgraph_operation_name {
                    OperationName::String => op_name.or_else(|| default.clone()),
                    OperationName::Hash => op_name.or_else(|| default.clone()).map(|op_name| {
                        let mut hasher = sha2::Sha256::new();
                        hasher.update(op_name.as_bytes());
                        let result = hasher.finalize();
                        hex::encode(result)
                    }),
                }
                .map(AttributeValue::String)
            }
            SubgraphSelector::SupergraphOperationName {
                supergraph_operation_name,
                default,
                ..
            } => {
                let op_name = request.context.get(OPERATION_NAME).ok().flatten();
                match supergraph_operation_name {
                    OperationName::String => op_name.or_else(|| default.clone()),
                    OperationName::Hash => op_name.or_else(|| default.clone()).map(|op_name| {
                        let mut hasher = sha2::Sha256::new();
                        hasher.update(op_name.as_bytes());
                        let result = hasher.finalize();
                        hex::encode(result)
                    }),
                }
                .map(AttributeValue::String)
            }
            SubgraphSelector::SubgraphOperationKind { .. } => AttributeValue::String(
                request
                    .operation_kind
                    .as_apollo_operation_type()
                    .to_string(),
            )
            .into(),
            SubgraphSelector::SupergraphOperationKind { .. } => {
                request.context.get(OPERATION_KIND).ok().flatten()
            }
            SubgraphSelector::SupergraphQuery { default, .. } => request
                .supergraph_request
                .body()
                .query
                .clone()
                .or_else(|| default.clone())
                .map(AttributeValue::String),
            SubgraphSelector::SubgraphQuery { default, .. } => request
                .subgraph_request
                .body()
                .query
                .clone()
                .or_else(|| default.clone())
                .map(AttributeValue::String),
            SubgraphSelector::SubgraphQueryVariable {
                subgraph_query_variable,
                default,
                ..
            } => request
                .subgraph_request
                .body()
                .variables
                .get(&ByteString::from(subgraph_query_variable.as_str()))
                .and_then(|v| serde_json::to_string(v).ok())
                .map(AttributeValue::String)
                .or_else(|| default.clone()),
            SubgraphSelector::SupergraphQueryVariable {
                supergraph_query_variable,
                default,
                ..
            } => request
                .supergraph_request
                .body()
                .variables
                .get(&ByteString::from(supergraph_query_variable.as_str()))
                .and_then(|v| serde_json::to_string(v).ok())
                .map(AttributeValue::String)
                .or_else(|| default.clone()),
            SubgraphSelector::SubgraphRequestHeader {
                subgraph_request_header,
                default,
                ..
            } => request
                .subgraph_request
                .headers()
                .get(subgraph_request_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            SubgraphSelector::SupergraphRequestHeader {
                supergraph_request_header,
                default,
                ..
            } => request
                .supergraph_request
                .headers()
                .get(supergraph_request_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            SubgraphSelector::RequestContext {
                request_context,
                default,
                ..
            } => request
                .context
                .get(request_context)
                .ok()
                .flatten()
                .or_else(|| default.clone()),
            SubgraphSelector::Baggage {
                baggage: baggage_name,
                default,
                ..
            } => {
                let span_context = Context::current();
                // I must clone the key because the otel API is bad
                let baggage = span_context.baggage().get(baggage_name.clone()).cloned();
                match baggage {
                    Some(baggage) => AttributeValue::from(baggage).into(),
                    None => default.clone(),
                }
            }
            SubgraphSelector::Env { env, default, .. } => std::env::var(env)
                .ok()
                .map(AttributeValue::String)
                .or_else(|| default.clone().map(AttributeValue::String)),
            // For response
            _ => None,
        }
    }

    fn on_response(&self, response: &subgraph::Response) -> Option<AttributeValue> {
        match self {
            SubgraphSelector::SubgraphResponseHeader {
                subgraph_response_header,
                default,
                ..
            } => response
                .response
                .headers()
                .get(subgraph_response_header)
                .and_then(|h| Some(AttributeValue::String(h.to_str().ok()?.to_string())))
                .or_else(|| default.clone()),
            SubgraphSelector::SubgraphResponseStatus {
                subgraph_response_status: response_status,
            } => match response_status {
                ResponseStatus::Code => Some(AttributeValue::I64(
                    response.response.status().as_u16() as i64,
                )),
                ResponseStatus::Reason => response
                    .response
                    .status()
                    .canonical_reason()
                    .map(|reason| AttributeValue::String(reason.to_string())),
            },
            SubgraphSelector::SubgraphResponseBody {
                subgraph_response_body,
                default,
                ..
            } => {
                let output = subgraph_response_body
                    .execute(response.response.body())
                    .ok()
                    .flatten()?;
                AttributeValue::try_from(output)
                    .ok()
                    .or_else(|| default.clone())
            }
            SubgraphSelector::ResponseContext {
                response_context,
                default,
                ..
            } => response
                .context
                .get(response_context)
                .ok()
                .flatten()
                .or_else(|| default.clone()),
            // For request
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::context::{OPERATION_KIND, OPERATION_NAME};
    use crate::graphql;
    use crate::plugins::telemetry::config::AttributeValue;
    use crate::plugins::telemetry::config_new::selectors::{
        OperationKind, OperationName, Query, ResponseStatus, RouterSelector, SubgraphSelector,
        SupergraphSelector, TraceIdFormat,
    };
    use crate::plugins::telemetry::config_new::GetAttribute;
    use http::StatusCode;
    use opentelemetry_api::baggage::BaggageExt;
    use opentelemetry_api::trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState,
    };
    use opentelemetry_api::{Context, KeyValue};
    use serde_json::json;
    use std::sync::Arc;
    use tracing::{span, subscriber};
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn router_request_header() {
        let selector = RouterSelector::RequestHeader {
            request_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_request(
                    &crate::services::RouterRequest::fake_builder()
                        .header("header_key", "header_value")
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_request(
                    &crate::services::RouterRequest::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_response(
                &crate::services::RouterResponse::fake_builder()
                    .context(crate::context::Context::default())
                    .header("header_key", "header_value")
                    .data(json!({}))
                    .build()
                    .unwrap()
            ),
            None
        );
    }
    #[test]
    fn router_response_header() {
        let selector = RouterSelector::ResponseHeader {
            response_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .header("header_key", "header_value")
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_request(
                &crate::services::RouterRequest::fake_builder()
                    .header("header_key", "header_value")
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn supergraph_request_header() {
        let selector = SupergraphSelector::RequestHeader {
            request_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_request(
                    &crate::services::SupergraphRequest::fake_builder()
                        .header("header_key", "header_value")
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_request(
                    &crate::services::SupergraphRequest::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_response(
                &crate::services::SupergraphResponse::fake_builder()
                    .header("header_key", "header_value")
                    .build()
                    .unwrap()
            ),
            None
        );
    }
    #[test]
    fn supergraph_response_header() {
        let selector = SupergraphSelector::ResponseHeader {
            response_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SupergraphResponse::fake_builder()
                        .header("header_key", "header_value")
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::SupergraphResponse::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .header("header_key", "header_value")
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn subgraph_supergraph_request_header() {
        let selector = SubgraphSelector::SupergraphRequestHeader {
            supergraph_request_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_request(
                    &crate::services::SubgraphRequest::fake_builder()
                        .supergraph_request(Arc::new(
                            http::Request::builder()
                                .header("header_key", "header_value")
                                .body(crate::request::Request::builder().build())
                                .unwrap()
                        ))
                        .build()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_request(&crate::services::SubgraphRequest::fake_builder().build())
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_response(
                &crate::services::SubgraphResponse::fake2_builder()
                    .header("header_key", "header_value")
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn subgraph_subgraph_request_header() {
        let selector = SubgraphSelector::SubgraphRequestHeader {
            subgraph_request_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_request(
                    &crate::services::SubgraphRequest::fake_builder()
                        .subgraph_request(
                            http::Request::builder()
                                .header("header_key", "header_value")
                                .body(graphql::Request::fake_builder().build())
                                .unwrap()
                        )
                        .build()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_request(&crate::services::SubgraphRequest::fake_builder().build())
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_response(
                &crate::services::SubgraphResponse::fake2_builder()
                    .header("header_key", "header_value")
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn subgraph_subgraph_response_header() {
        let selector = SubgraphSelector::SubgraphResponseHeader {
            subgraph_response_header: "header_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake2_builder()
                        .header("header_key", "header_value")
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "header_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake2_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .subgraph_request(
                        http::Request::builder()
                            .header("header_key", "header_value")
                            .body(graphql::Request::fake_builder().build())
                            .unwrap()
                    )
                    .build()
            ),
            None
        );
    }

    #[test]
    fn router_response_context() {
        let selector = RouterSelector::ResponseContext {
            response_context: "context_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        let context = crate::context::Context::new();
        let _ = context.insert("context_key".to_string(), "context_value".to_string());
        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .context(context.clone())
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "context_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );
        assert_eq!(
            selector.on_request(
                &crate::services::RouterRequest::fake_builder()
                    .context(context)
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn supergraph_request_context() {
        let selector = SupergraphSelector::RequestContext {
            request_context: "context_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        let context = crate::context::Context::new();
        let _ = context.insert("context_key".to_string(), "context_value".to_string());
        assert_eq!(
            selector
                .on_request(
                    &crate::services::SupergraphRequest::fake_builder()
                        .context(context.clone())
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "context_value".into()
        );

        assert_eq!(
            selector
                .on_request(
                    &crate::services::SupergraphRequest::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );
        assert_eq!(
            selector.on_response(
                &crate::services::SupergraphResponse::fake_builder()
                    .context(context)
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn supergraph_response_context() {
        let selector = SupergraphSelector::ResponseContext {
            response_context: "context_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        let context = crate::context::Context::new();
        let _ = context.insert("context_key".to_string(), "context_value".to_string());
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SupergraphResponse::fake_builder()
                        .context(context.clone())
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "context_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::SupergraphResponse::fake_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context)
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn subgraph_request_context() {
        let selector = SubgraphSelector::RequestContext {
            request_context: "context_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        let context = crate::context::Context::new();
        let _ = context.insert("context_key".to_string(), "context_value".to_string());
        assert_eq!(
            selector
                .on_request(
                    &crate::services::SubgraphRequest::fake_builder()
                        .context(context.clone())
                        .build()
                )
                .unwrap(),
            "context_value".into()
        );

        assert_eq!(
            selector
                .on_request(&crate::services::SubgraphRequest::fake_builder().build())
                .unwrap(),
            "defaulted".into()
        );
        assert_eq!(
            selector.on_response(
                &crate::services::SubgraphResponse::fake2_builder()
                    .context(context)
                    .build()
                    .unwrap()
            ),
            None
        );
    }

    #[test]
    fn subgraph_response_context() {
        let selector = SubgraphSelector::ResponseContext {
            response_context: "context_key".to_string(),
            redact: None,
            default: Some("defaulted".into()),
        };
        let context = crate::context::Context::new();
        let _ = context.insert("context_key".to_string(), "context_value".to_string());
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake2_builder()
                        .context(context.clone())
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "context_value".into()
        );

        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake2_builder()
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "defaulted".into()
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context)
                    .build()
            ),
            None
        );
    }

    #[test]
    fn router_baggage() {
        let subscriber = tracing_subscriber::registry().with(tracing_opentelemetry::layer());
        subscriber::with_default(subscriber, || {
            let span = span!(tracing::Level::INFO, "test");
            let _guard = span.enter();
            let selector = RouterSelector::Baggage {
                baggage: "baggage_key".to_string(),
                redact: None,
                default: Some("defaulted".into()),
            };
            assert_eq!(
                selector
                    .on_request(
                        &crate::services::RouterRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                "defaulted".into()
            );

            let _outer_guard = span
                .context()
                .with_baggage(vec![KeyValue::new("baggage_key", "baggage_value")])
                .attach();

            assert_eq!(
                selector
                    .on_request(
                        &crate::services::RouterRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                "baggage_value".into()
            );
        });
    }

    #[test]
    fn supergraph_baggage() {
        let subscriber = tracing_subscriber::registry().with(tracing_opentelemetry::layer());
        subscriber::with_default(subscriber, || {
            let span = span!(tracing::Level::INFO, "test");
            let _guard = span.enter();
            let selector = SupergraphSelector::Baggage {
                baggage: "baggage_key".to_string(),
                redact: None,
                default: Some("defaulted".into()),
            };
            assert_eq!(
                selector
                    .on_request(
                        &crate::services::SupergraphRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                "defaulted".into()
            );

            let _outer_guard = span
                .context()
                .with_baggage(vec![KeyValue::new("baggage_key", "baggage_value")])
                .attach();

            assert_eq!(
                selector
                    .on_request(
                        &crate::services::SupergraphRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                "baggage_value".into()
            );
        });
    }

    #[test]
    fn subgraph_baggage() {
        let subscriber = tracing_subscriber::registry().with(tracing_opentelemetry::layer());
        subscriber::with_default(subscriber, || {
            let span = span!(tracing::Level::INFO, "test");
            let _guard = span.enter();
            let selector = SubgraphSelector::Baggage {
                baggage: "baggage_key".to_string(),
                redact: None,
                default: Some("defaulted".into()),
            };
            assert_eq!(
                selector
                    .on_request(&crate::services::SubgraphRequest::fake_builder().build())
                    .unwrap(),
                "defaulted".into()
            );

            let _outer_guard = span
                .context()
                .with_baggage(vec![KeyValue::new("baggage_key", "baggage_value")])
                .attach();

            assert_eq!(
                selector
                    .on_request(&crate::services::SubgraphRequest::fake_builder().build())
                    .unwrap(),
                "baggage_value".into()
            );
        });
    }

    #[test]
    fn router_trace_id() {
        let subscriber = tracing_subscriber::registry().with(tracing_opentelemetry::layer());

        subscriber::with_default(subscriber, || {
            let span_context = SpanContext::new(
                TraceId::from_u128(42),
                SpanId::from_u64(42),
                TraceFlags::default(),
                true,
                TraceState::default(),
            );
            let span = span!(tracing::Level::INFO, "test");
            let _guard = span.enter();
            let selector = RouterSelector::TraceId {
                trace_id: TraceIdFormat::OpenTelemetry,
            };
            // No span context
            assert_eq!(
                selector.on_request(
                    &crate::services::RouterRequest::fake_builder()
                        .build()
                        .unwrap(),
                ),
                None
            );
            // Context set
            let _context = Context::current()
                .with_remote_span_context(span_context)
                .attach();
            assert_eq!(
                selector
                    .on_request(
                        &crate::services::RouterRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                "0000000000000000000000000000002a".into()
            );

            let selector = RouterSelector::TraceId {
                trace_id: TraceIdFormat::Datadog,
            };

            assert_eq!(
                selector
                    .on_request(
                        &crate::services::RouterRequest::fake_builder()
                            .build()
                            .unwrap(),
                    )
                    .unwrap(),
                AttributeValue::U128(42)
            );
        });
    }

    #[test]
    fn router_env() {
        let selector = RouterSelector::Env {
            env: "SELECTOR_ENV_VARIABLE".to_string(),
            redact: None,
            default: Some("defaulted".to_string()),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::RouterRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("defaulted".into())
        );
        // Env set
        std::env::set_var("SELECTOR_ENV_VARIABLE", "env_value");

        assert_eq!(
            selector.on_request(
                &crate::services::RouterRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("env_value".into())
        );
    }

    #[test]
    fn supergraph_env() {
        let selector = SupergraphSelector::Env {
            env: "SELECTOR_SUPERGRAPH_ENV_VARIABLE".to_string(),
            redact: None,
            default: Some("defaulted".to_string()),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("defaulted".into())
        );
        // Env set
        std::env::set_var("SELECTOR_SUPERGRAPH_ENV_VARIABLE", "env_value");

        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("env_value".into())
        );
        std::env::remove_var("SELECTOR_SUPERGRAPH_ENV_VARIABLE");
    }

    #[test]
    fn subgraph_env() {
        let selector = SubgraphSelector::Env {
            env: "SELECTOR_SUBGRAPH_ENV_VARIABLE".to_string(),
            redact: None,
            default: Some("defaulted".to_string()),
        };
        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build()),
            Some("defaulted".into())
        );
        // Env set
        std::env::set_var("SELECTOR_SUBGRAPH_ENV_VARIABLE", "env_value");

        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build()),
            Some("env_value".into())
        );
        std::env::remove_var("SELECTOR_SUBGRAPH_ENV_VARIABLE");
    }

    #[test]
    fn supergraph_operation_kind() {
        let selector = SupergraphSelector::OperationKind {
            operation_kind: OperationKind::String,
        };
        let context = crate::context::Context::new();
        let _ = context.insert(OPERATION_KIND, "query".to_string());
        // For now operation kind is contained in context
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context)
                    .build()
                    .unwrap(),
            ),
            Some("query".into())
        );
    }

    #[test]
    fn subgraph_operation_kind() {
        let selector = SubgraphSelector::SupergraphOperationKind {
            supergraph_operation_kind: OperationKind::String,
        };
        let context = crate::context::Context::new();
        let _ = context.insert(OPERATION_KIND, "query".to_string());
        // For now operation kind is contained in context
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context)
                    .build(),
            ),
            Some("query".into())
        );
    }

    #[test]
    fn supergraph_operation_name_string() {
        let selector = SupergraphSelector::OperationName {
            operation_name: OperationName::String,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        let context = crate::context::Context::new();
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context.clone())
                    .build()
                    .unwrap(),
            ),
            Some("defaulted".into())
        );
        let _ = context.insert(OPERATION_NAME, "topProducts".to_string());
        // For now operation kind is contained in context
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context)
                    .build()
                    .unwrap(),
            ),
            Some("topProducts".into())
        );
    }

    #[test]
    fn subgraph_supergraph_operation_name_string() {
        let selector = SubgraphSelector::SupergraphOperationName {
            supergraph_operation_name: OperationName::String,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        let context = crate::context::Context::new();
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context.clone())
                    .build(),
            ),
            Some("defaulted".into())
        );
        let _ = context.insert(OPERATION_NAME, "topProducts".to_string());
        // For now operation kind is contained in context
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context)
                    .build(),
            ),
            Some("topProducts".into())
        );
    }

    #[test]
    fn subgraph_subgraph_operation_name_string() {
        let selector = SubgraphSelector::SubgraphOperationName {
            subgraph_operation_name: OperationName::String,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build(),),
            Some("defaulted".into())
        );
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .subgraph_request(
                        ::http::Request::builder()
                            .uri("http://localhost/graphql")
                            .body(
                                graphql::Request::fake_builder()
                                    .operation_name("topProducts")
                                    .build()
                            )
                            .unwrap()
                    )
                    .build(),
            ),
            Some("topProducts".into())
        );
    }

    #[test]
    fn supergraph_operation_name_hash() {
        let selector = SupergraphSelector::OperationName {
            operation_name: OperationName::Hash,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        let context = crate::context::Context::new();
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context.clone())
                    .build()
                    .unwrap(),
            ),
            Some("96294f50edb8f006f6b0a2dadae50d3c521e9841d07d6395d91060c8ccfed7f0".into())
        );

        let _ = context.insert(OPERATION_NAME, "topProducts".to_string());
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .context(context)
                    .build()
                    .unwrap(),
            ),
            Some("bd141fca26094be97c30afd42e9fc84755b252e7052d8c992358319246bd555a".into())
        );
    }

    #[test]
    fn subgraph_supergraph_operation_name_hash() {
        let selector = SubgraphSelector::SupergraphOperationName {
            supergraph_operation_name: OperationName::Hash,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        let context = crate::context::Context::new();
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context.clone())
                    .build(),
            ),
            Some("96294f50edb8f006f6b0a2dadae50d3c521e9841d07d6395d91060c8ccfed7f0".into())
        );

        let _ = context.insert(OPERATION_NAME, "topProducts".to_string());
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .context(context)
                    .build(),
            ),
            Some("bd141fca26094be97c30afd42e9fc84755b252e7052d8c992358319246bd555a".into())
        );
    }

    #[test]
    fn subgraph_subgraph_operation_name_hash() {
        let selector = SubgraphSelector::SubgraphOperationName {
            subgraph_operation_name: OperationName::Hash,
            redact: None,
            default: Some("defaulted".to_string()),
        };
        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build()),
            Some("96294f50edb8f006f6b0a2dadae50d3c521e9841d07d6395d91060c8ccfed7f0".into())
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .subgraph_request(
                        ::http::Request::builder()
                            .uri("http://localhost/graphql")
                            .body(
                                graphql::Request::fake_builder()
                                    .operation_name("topProducts")
                                    .build()
                            )
                            .unwrap()
                    )
                    .build()
            ),
            Some("bd141fca26094be97c30afd42e9fc84755b252e7052d8c992358319246bd555a".into())
        );
    }

    #[test]
    fn supergraph_query() {
        let selector = SupergraphSelector::Query {
            query: Query::String,
            redact: None,
            default: Some("default".to_string()),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .query("topProducts{name}")
                    .build()
                    .unwrap(),
            ),
            Some("topProducts{name}".into())
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("default".into())
        );
    }

    #[test]
    fn subgraph_supergraph_query() {
        let selector = SubgraphSelector::SupergraphQuery {
            supergraph_query: Query::String,
            redact: None,
            default: Some("default".to_string()),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .supergraph_request(Arc::new(
                        http::Request::builder()
                            .body(
                                graphql::Request::fake_builder()
                                    .query("topProducts{name}")
                                    .build()
                            )
                            .unwrap()
                    ))
                    .build(),
            ),
            Some("topProducts{name}".into())
        );

        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build(),),
            Some("default".into())
        );
    }

    #[test]
    fn subgraph_subgraph_query() {
        let selector = SubgraphSelector::SubgraphQuery {
            subgraph_query: Query::String,
            redact: None,
            default: Some("default".to_string()),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .subgraph_request(
                        http::Request::builder()
                            .body(
                                graphql::Request::fake_builder()
                                    .query("topProducts{name}")
                                    .build()
                            )
                            .unwrap()
                    )
                    .build(),
            ),
            Some("topProducts{name}".into())
        );

        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build(),),
            Some("default".into())
        );
    }

    #[test]
    fn router_response_status_code() {
        let selector = RouterSelector::ResponseStatus {
            response_status: ResponseStatus::Code,
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .status_code(StatusCode::NO_CONTENT)
                        .build()
                        .unwrap()
                )
                .unwrap(),
            AttributeValue::I64(204)
        );
    }

    #[test]
    fn subgraph_subgraph_response_status_code() {
        let selector = SubgraphSelector::SubgraphResponseStatus {
            subgraph_response_status: ResponseStatus::Code,
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake_builder()
                        .status_code(StatusCode::NO_CONTENT)
                        .build()
                )
                .unwrap(),
            AttributeValue::I64(204)
        );
    }

    #[test]
    fn router_response_status_reason() {
        let selector = RouterSelector::ResponseStatus {
            response_status: ResponseStatus::Reason,
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::RouterResponse::fake_builder()
                        .status_code(StatusCode::NO_CONTENT)
                        .build()
                        .unwrap()
                )
                .unwrap(),
            "No Content".into()
        );
    }

    #[test]
    fn subgraph_subgraph_response_status_reason() {
        let selector = SubgraphSelector::SubgraphResponseStatus {
            subgraph_response_status: ResponseStatus::Reason,
        };
        assert_eq!(
            selector
                .on_response(
                    &crate::services::SubgraphResponse::fake_builder()
                        .status_code(StatusCode::NO_CONTENT)
                        .build()
                )
                .unwrap(),
            "No Content".into()
        );
    }

    #[test]
    fn supergraph_query_variable() {
        let selector = SupergraphSelector::QueryVariable {
            query_variable: "key".to_string(),
            redact: None,
            default: Some(AttributeValue::String("default".to_string())),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .variable("key", "value")
                    .build()
                    .unwrap(),
            ),
            Some("\"value\"".into())
        );

        assert_eq!(
            selector.on_request(
                &crate::services::SupergraphRequest::fake_builder()
                    .build()
                    .unwrap(),
            ),
            Some("default".into())
        );
    }

    #[test]
    fn subgraph_supergraph_query_variable() {
        let selector = SubgraphSelector::SupergraphQueryVariable {
            supergraph_query_variable: "key".to_string(),
            redact: None,
            default: Some(AttributeValue::String("default".to_string())),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .supergraph_request(Arc::new(
                        http::Request::builder()
                            .body(
                                graphql::Request::fake_builder()
                                    .variable("key", "value")
                                    .build()
                            )
                            .unwrap()
                    ))
                    .build(),
            ),
            Some("\"value\"".into())
        );

        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build(),),
            Some("default".into())
        );
    }

    #[test]
    fn subgraph_subgraph_query_variable() {
        let selector = SubgraphSelector::SubgraphQueryVariable {
            subgraph_query_variable: "key".to_string(),
            redact: None,
            default: Some(AttributeValue::String("default".to_string())),
        };
        assert_eq!(
            selector.on_request(
                &crate::services::SubgraphRequest::fake_builder()
                    .subgraph_request(
                        http::Request::builder()
                            .body(
                                graphql::Request::fake_builder()
                                    .variable("key", "value")
                                    .build()
                            )
                            .unwrap()
                    )
                    .build(),
            ),
            Some("\"value\"".into())
        );

        assert_eq!(
            selector.on_request(&crate::services::SubgraphRequest::fake_builder().build(),),
            Some("default".into())
        );
    }
}
