use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use tauri_plugin_mihomo::models::{
    DelayHistory, Proxies, Proxy, ProxyProvider, ProxyProviders, ProxyType, VehicleType,
};

use super::delay_display::{DEFAULT_DELAY_DISPLAY_PERCENT, DelayDisplayContext};

pub struct ProxyViewInput {
    pub runtime_group_order: Vec<String>,
    pub proxies: Proxies,
    pub providers: Option<ProxyProviders>,
    pub delay_display: DelayDisplayContext,
}

pub struct ProxyViewBuilder;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyViewV1 {
    pub schema_version: u8,
    pub order_source: ProxyViewOrderSource,
    pub provider_state: ProxyViewProviderState,
    pub global: Option<ProxyGroupView>,
    pub direct: Option<String>,
    pub groups: Vec<ProxyGroupView>,
    pub records: BTreeMap<String, ProxyNodeView>,
    pub standalone: Vec<String>,
    pub providers: Vec<ProxyProviderView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyViewOrderSource {
    Runtime,
    Fallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyViewProviderState {
    Ready,
    Unavailable,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ProxyCapabilities {
    pub udp: bool,
    pub xudp: bool,
    pub tfo: bool,
    pub mptcp: bool,
    pub smux: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyGroupView {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub alive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_url: Option<String>,
    pub history: Vec<DelayHistory>,
    pub delay_display_percent: u16,
    #[serde(flatten)]
    pub capabilities: ProxyCapabilities,
    pub members: Vec<ProxyMemberRef>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyNodeView {
    pub record_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub alive: bool,
    pub history: Vec<DelayHistory>,
    pub delay_display_percent: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_url: Option<String>,
    #[serde(flatten)]
    pub capabilities: ProxyCapabilities,
    pub source: ProxyNodeSource,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProxyNodeSource {
    Core {
        #[serde(rename = "proxyName")]
        proxy_name: String,
    },
    Provider {
        #[serde(rename = "providerName")]
        provider_name: String,
        #[serde(rename = "proxyName")]
        proxy_name: String,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProxyMemberRef {
    Group {
        name: String,
    },
    Node {
        name: String,
        #[serde(rename = "recordId")]
        record_id: String,
    },
    Unresolved {
        name: String,
        reason: ProxyMemberUnresolvedReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ProxyMemberUnresolvedReason {
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "ambiguous")]
    Ambiguous,
    #[serde(rename = "provider-unavailable")]
    ProviderUnavailable,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyProviderView {
    pub name: String,
    pub vehicle_type: ProxyProviderVehicleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_info: Option<ProxySubscriptionInfo>,
    pub proxy_record_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ProxyProviderVehicleType {
    #[serde(rename = "HTTP")]
    Http,
    #[serde(rename = "File")]
    File,
    #[serde(rename = "Inline")]
    Inline,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ProxySubscriptionInfo {
    pub upload: i64,
    pub download: i64,
    pub total: i64,
    pub expire: i64,
}

struct MemberResolver<'a> {
    group_names: BTreeSet<String>,
    core_node_ids: &'a BTreeMap<String, String>,
    provider_candidates: &'a BTreeMap<String, Vec<String>>,
    provider_available: bool,
}

struct GroupDelayDisplay<'a> {
    now_by_group: &'a BTreeMap<String, Option<String>>,
    core_node_ids: &'a BTreeMap<String, String>,
    provider_candidates: &'a BTreeMap<String, Vec<String>>,
    records: &'a BTreeMap<String, ProxyNodeView>,
    default_percent: u16,
}

impl GroupDelayDisplay<'_> {
    fn percent_for_group(&self, name: &str) -> u16 {
        self.resolve_member_name(name, &mut BTreeSet::new())
    }

    fn resolve_member_name(&self, name: &str, visiting: &mut BTreeSet<String>) -> u16 {
        if let Some(record_id) = self.core_node_ids.get(name) {
            return self
                .records
                .get(record_id)
                .map_or(self.default_percent, |record| record.delay_display_percent);
        }

        match self.provider_candidates.get(name).map(Vec::as_slice) {
            Some([record_id]) => {
                return self
                    .records
                    .get(record_id)
                    .map_or(self.default_percent, |record| record.delay_display_percent);
            }
            Some(_) => return self.default_percent,
            None => {}
        }

        let Some(Some(now)) = self.now_by_group.get(name) else {
            return self.default_percent;
        };
        if !visiting.insert(name.to_owned()) {
            return self.default_percent;
        }
        let percent = self.resolve_member_name(now, visiting);
        visiting.remove(name);
        percent
    }
}

impl MemberResolver<'_> {
    fn resolve(&self, name: String) -> ProxyMemberRef {
        if self.group_names.contains(&name) {
            ProxyMemberRef::Group { name }
        } else if let Some(record_id) = self.core_node_ids.get(&name) {
            ProxyMemberRef::Node {
                name,
                record_id: record_id.clone(),
            }
        } else if !self.provider_available {
            ProxyMemberRef::Unresolved {
                name,
                reason: ProxyMemberUnresolvedReason::ProviderUnavailable,
            }
        } else {
            match self.provider_candidates.get(&name).map(Vec::as_slice) {
                Some([record_id]) => ProxyMemberRef::Node {
                    name,
                    record_id: record_id.clone(),
                },
                None => ProxyMemberRef::Unresolved {
                    name,
                    reason: ProxyMemberUnresolvedReason::Missing,
                },
                Some(_) => ProxyMemberRef::Unresolved {
                    name,
                    reason: ProxyMemberUnresolvedReason::Ambiguous,
                },
            }
        }
    }
}

impl ProxyViewBuilder {
    pub fn build(input: ProxyViewInput) -> ProxyViewV1 {
        let ProxyViewInput {
            runtime_group_order,
            proxies,
            providers,
            delay_display,
        } = input;
        let provider_state = if providers.is_some() {
            ProxyViewProviderState::Ready
        } else {
            ProxyViewProviderState::Unavailable
        };
        let (mut core_groups, core_nodes) = partition_core(proxies);
        let now_by_group = core_groups
            .iter()
            .map(|(name, group)| (name.clone(), group.now.clone()))
            .collect();
        let (mut records, core_node_ids) = build_core_records(core_nodes, delay_display.profile_percent());
        let (providers, provider_candidates) = build_provider_records(providers, &mut records, &delay_display);
        let group_delay_display = GroupDelayDisplay {
            now_by_group: &now_by_group,
            core_node_ids: &core_node_ids,
            provider_candidates: &provider_candidates,
            records: &records,
            default_percent: DEFAULT_DELAY_DISPLAY_PERCENT,
        };
        let resolver = MemberResolver {
            group_names: core_groups.keys().cloned().collect(),
            core_node_ids: &core_node_ids,
            provider_candidates: &provider_candidates,
            provider_available: provider_state == ProxyViewProviderState::Ready,
        };

        let global = core_groups
            .remove("GLOBAL")
            .map(|proxy| build_group("GLOBAL".to_owned(), proxy, &resolver, &group_delay_display));
        let (groups, order_source) =
            build_ordered_groups(core_groups, runtime_group_order, &resolver, &group_delay_display);
        let direct = core_node_ids.get("DIRECT").cloned();
        let standalone = build_standalone(&core_node_ids);

        ProxyViewV1 {
            schema_version: 1,
            order_source,
            provider_state,
            global,
            direct,
            groups,
            records,
            standalone,
            providers,
        }
    }
}

fn partition_core(proxies: Proxies) -> (BTreeMap<String, Proxy>, BTreeMap<String, Proxy>) {
    let mut groups = BTreeMap::new();
    let mut nodes = BTreeMap::new();

    for (name, proxy) in proxies.proxies {
        if proxy.all.is_some() {
            groups.insert(name, proxy);
        } else {
            nodes.insert(name, proxy);
        }
    }

    (groups, nodes)
}

fn build_core_records(
    core_nodes: BTreeMap<String, Proxy>,
    delay_display_percent: u16,
) -> (BTreeMap<String, ProxyNodeView>, BTreeMap<String, String>) {
    let mut records = BTreeMap::new();
    let mut ids = BTreeMap::new();

    for (index, (name, proxy)) in core_nodes.into_iter().enumerate() {
        let record_id = format!("c:{index}");
        ids.insert(name.clone(), record_id.clone());
        records.insert(
            record_id.clone(),
            build_node(
                record_id,
                name.clone(),
                proxy,
                ProxyNodeSource::Core { proxy_name: name },
                delay_display_percent,
            ),
        );
    }

    (records, ids)
}

fn build_provider_records(
    providers: Option<ProxyProviders>,
    records: &mut BTreeMap<String, ProxyNodeView>,
    delay_display: &DelayDisplayContext,
) -> (Vec<ProxyProviderView>, BTreeMap<String, Vec<String>>) {
    let providers = providers
        .map(|providers| providers.providers.into_iter().collect::<BTreeMap<_, _>>())
        .unwrap_or_default();
    let mut views = Vec::new();
    let mut candidates = BTreeMap::new();

    for (provider_name, provider) in providers {
        let ProxyProvider {
            vehicle_type,
            proxies,
            updated_at,
            subscription_info,
            ..
        } = provider;
        let vehicle_type = match vehicle_type {
            VehicleType::HTTP => ProxyProviderVehicleType::Http,
            VehicleType::File => ProxyProviderVehicleType::File,
            VehicleType::Inline => ProxyProviderVehicleType::Inline,
            _ => continue,
        };
        let provider_index = views.len();
        let mut proxy_record_ids = Vec::new();

        for (member_index, proxy) in proxies.into_iter().enumerate() {
            let record_id = format!("p:{provider_index}:{member_index}");
            let proxy_name = proxy.name.clone();
            let node_name = proxy.name.clone();
            candidates
                .entry(proxy_name.clone())
                .or_insert_with(Vec::new)
                .push(record_id.clone());
            records.insert(
                record_id.clone(),
                build_node(
                    record_id.clone(),
                    node_name,
                    proxy,
                    ProxyNodeSource::Provider {
                        provider_name: provider_name.clone(),
                        proxy_name,
                    },
                    delay_display.node_percent(Some(&provider_name)),
                ),
            );
            proxy_record_ids.push(record_id);
        }

        views.push(ProxyProviderView {
            name: provider_name,
            vehicle_type,
            updated_at,
            subscription_info: subscription_info.map(|subscription_info| ProxySubscriptionInfo {
                upload: subscription_info.upload,
                download: subscription_info.download,
                total: subscription_info.total,
                expire: subscription_info.expire,
            }),
            proxy_record_ids,
        });
    }

    (views, candidates)
}

fn build_group(
    name: String,
    proxy: Proxy,
    resolver: &MemberResolver<'_>,
    group_delay_display: &GroupDelayDisplay<'_>,
) -> ProxyGroupView {
    let Proxy {
        all,
        fixed,
        hidden,
        icon,
        now,
        test_url,
        alive,
        history,
        udp,
        xudp,
        tfo,
        mptcp,
        smux,
        proxy_type,
        ..
    } = proxy;

    let delay_display_percent = group_delay_display.percent_for_group(&name);

    ProxyGroupView {
        name,
        proxy_type,
        alive,
        now,
        fixed,
        hidden,
        icon,
        test_url,
        history,
        delay_display_percent,
        capabilities: ProxyCapabilities {
            udp,
            xudp,
            tfo,
            mptcp,
            smux,
        },
        members: all
            .unwrap_or_default()
            .into_iter()
            .map(|name| resolver.resolve(name))
            .collect(),
    }
}

fn build_node(
    record_id: String,
    name: String,
    proxy: Proxy,
    source: ProxyNodeSource,
    delay_display_percent: u16,
) -> ProxyNodeView {
    let Proxy {
        id,
        hidden,
        icon,
        test_url,
        alive,
        history,
        udp,
        xudp,
        tfo,
        mptcp,
        smux,
        proxy_type,
        ..
    } = proxy;

    ProxyNodeView {
        record_id,
        name,
        proxy_type,
        alive,
        history,
        delay_display_percent,
        id,
        hidden,
        icon,
        test_url,
        capabilities: ProxyCapabilities {
            udp,
            xudp,
            tfo,
            mptcp,
            smux,
        },
        source,
    }
}

fn build_ordered_groups(
    mut core_groups: BTreeMap<String, Proxy>,
    runtime_group_order: Vec<String>,
    resolver: &MemberResolver<'_>,
    group_delay_display: &GroupDelayDisplay<'_>,
) -> (Vec<ProxyGroupView>, ProxyViewOrderSource) {
    let mut groups = Vec::with_capacity(core_groups.len());
    for name in runtime_group_order {
        if let Some(proxy) = core_groups.remove(&name) {
            groups.push(build_group(name, proxy, resolver, group_delay_display));
        }
    }

    let order_source = if groups.is_empty() {
        ProxyViewOrderSource::Fallback
    } else {
        ProxyViewOrderSource::Runtime
    };
    groups.extend(
        core_groups
            .into_iter()
            .map(|(name, proxy)| build_group(name, proxy, resolver, group_delay_display)),
    );
    (groups, order_source)
}

fn build_standalone(core_node_ids: &BTreeMap<String, String>) -> Vec<String> {
    let mut standalone = ["DIRECT", "REJECT"]
        .into_iter()
        .filter_map(|name| core_node_ids.get(name).cloned())
        .collect::<Vec<_>>();
    standalone.extend(
        core_node_ids
            .iter()
            .filter(|(name, _)| name.as_str() != "DIRECT" && name.as_str() != "REJECT")
            .map(|(_, record_id)| record_id.clone()),
    );
    standalone
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use tauri_plugin_mihomo::models::{
        Proxies, Proxy, ProxyProvider, ProxyProviders, ProxyType, SubScriptionInfo, VehicleType,
    };

    use super::{
        ProxyMemberRef, ProxyMemberUnresolvedReason, ProxyNodeSource, ProxyViewBuilder, ProxyViewInput,
        ProxyViewOrderSource, ProxyViewProviderState,
    };
    use crate::core::delay_display::{DEFAULT_DELAY_DISPLAY_PERCENT, DelayDisplayContext, SLOVE_DELAY_DISPLAY_PERCENT};

    fn node(name: &str) -> Proxy {
        Proxy {
            name: name.to_owned(),
            proxy_type: ProxyType::Shadowsocks,
            alive: true,
            udp: true,
            ..Proxy::default()
        }
    }

    fn group(name: &str, members: &[&str]) -> Proxy {
        Proxy {
            name: format!("ignored-{name}"),
            proxy_type: ProxyType::Selector,
            alive: true,
            all: Some(members.iter().map(|name| (*name).to_owned()).collect()),
            ..Proxy::default()
        }
    }

    fn ordered_input(reverse_insert: bool) -> ProxyViewInput {
        let entries = [
            ("Zulu", group("Zulu", &["node-b", "node-b"])),
            ("Alpha", group("Alpha", &["node-a"])),
            ("node-b", node("wrong-node-b")),
            ("node-a", node("wrong-node-a")),
            ("A-before-direct", node("wrong-before-direct")),
            ("DIRECT", node("wrong-direct")),
            ("REJECT", node("wrong-reject")),
        ];
        let mut proxies = HashMap::new();
        if reverse_insert {
            proxies.extend(entries.into_iter().rev().map(|(name, proxy)| (name.to_owned(), proxy)));
        } else {
            proxies.extend(entries.into_iter().map(|(name, proxy)| (name.to_owned(), proxy)));
        }

        ProxyViewInput {
            runtime_group_order: vec!["Zulu".into(), "Alpha".into()],
            proxies: Proxies { proxies },
            providers: None,
            delay_display: Default::default(),
        }
    }

    fn providers_fixture(entries: Vec<(&str, VehicleType, &[&str])>) -> ProxyProviders {
        let providers = entries
            .into_iter()
            .map(|(provider_key, vehicle_type, names)| {
                let provider = ProxyProvider {
                    name: format!("ignored-{provider_key}"),
                    vehicle_type,
                    proxies: names.iter().map(|name| node(name)).collect(),
                    ..ProxyProvider::default()
                };
                (provider_key.to_owned(), provider)
            })
            .collect();

        ProxyProviders { providers }
    }

    fn input_with_members_and_duplicate_providers() -> ProxyViewInput {
        let proxies = HashMap::from([("Group".to_owned(), group("ignored", &["duplicate"]))]);
        ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies { proxies },
            providers: Some(providers_fixture(vec![
                ("a", VehicleType::HTTP, &["duplicate"]),
                ("b", VehicleType::File, &["duplicate"]),
            ])),
            delay_display: Default::default(),
        }
    }

    fn input_with_provider_member(providers: Option<ProxyProviders>) -> ProxyViewInput {
        let proxies = HashMap::from([("Group".to_owned(), group("ignored", &["provider-only"]))]);
        ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies { proxies },
            providers,
            delay_display: Default::default(),
        }
    }

    fn input_with_provider_duplicates() -> ProxyViewInput {
        ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies::default(),
            providers: Some(providers_fixture(vec![
                ("z-provider", VehicleType::HTTP, &["z-node"]),
                ("unsupported", VehicleType::Compatible, &["ignored"]),
                ("a-provider", VehicleType::File, &["same", "same"]),
            ])),
            delay_display: Default::default(),
        }
    }

    fn serde_input_with_provider() -> ProxyViewInput {
        let provider = ProxyProvider {
            name: "ignored-object-name".to_owned(),
            vehicle_type: VehicleType::HTTP,
            updated_at: None,
            subscription_info: Some(SubScriptionInfo {
                upload: 1,
                download: 2,
                total: 3,
                expire: 4,
            }),
            ..ProxyProvider::default()
        };

        ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies::default(),
            providers: Some(ProxyProviders {
                providers: HashMap::from([("provider-key".to_owned(), provider)]),
            }),
            delay_display: Default::default(),
        }
    }

    #[test]
    fn ordering_and_core_records_are_deterministic() {
        let first = ProxyViewBuilder::build(ordered_input(false));
        let second = ProxyViewBuilder::build(ordered_input(true));

        assert_eq!(first, second);
        assert_eq!(first.order_source, ProxyViewOrderSource::Runtime);
        assert_eq!(
            first.groups.iter().map(|group| group.name.as_str()).collect::<Vec<_>>(),
            ["Zulu", "Alpha"]
        );
        assert_eq!(first.direct.as_deref(), Some("c:1"));
        assert_eq!(first.standalone, ["c:1", "c:2", "c:0", "c:3", "c:4"]);
        assert_eq!(first.records["c:0"].name, "A-before-direct");
        assert_eq!(first.records["c:1"].name, "DIRECT");
        assert_eq!(first.records["c:4"].name, "node-b");
        assert_eq!(first.groups[0].members.len(), 2);
        assert!(matches!(
            &first.groups[0].members[0],
            ProxyMemberRef::Node { name, .. } if name == "node-b"
        ));
        assert_eq!(first.groups[0].members[0], first.groups[0].members[1]);
    }

    #[test]
    fn fallback_and_unmatched_groups_are_stable() {
        let mut input = ordered_input(false);
        input.runtime_group_order = vec![];
        let view = ProxyViewBuilder::build(input);
        assert_eq!(view.order_source, ProxyViewOrderSource::Fallback);
        assert_eq!(
            view.groups.iter().map(|group| group.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "Zulu"]
        );

        let mut input = ordered_input(false);
        input.runtime_group_order = vec!["Missing".into()];
        let view = ProxyViewBuilder::build(input);
        assert_eq!(view.order_source, ProxyViewOrderSource::Fallback);
        assert_eq!(
            view.groups.iter().map(|group| group.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "Zulu"]
        );

        let mut input = ordered_input(false);
        input.runtime_group_order = vec!["Zulu".into(), "Zulu".into()];
        let view = ProxyViewBuilder::build(input);
        assert_eq!(
            view.groups.iter().map(|group| group.name.as_str()).collect::<Vec<_>>(),
            ["Zulu", "Alpha"]
        );
    }

    #[test]
    fn global_and_member_resolution_follow_the_required_priority() {
        let mut proxies = HashMap::new();
        proxies.insert(
            "GLOBAL".into(),
            group("wrong", &["Nested", "core", "provider-only", "missing"]),
        );
        proxies.insert("Nested".into(), group("wrong", &[]));
        proxies.insert("core".into(), node("wrong"));
        let providers = providers_fixture(vec![
            (
                "a",
                VehicleType::HTTP,
                &["Nested", "core", "provider-only", "duplicate"],
            ),
            ("b", VehicleType::File, &["duplicate"]),
        ]);

        let view = ProxyViewBuilder::build(ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies { proxies },
            providers: Some(providers),
            delay_display: Default::default(),
        });
        let members = &view.global.as_ref().expect("GLOBAL").members;

        assert!(matches!(members[0], ProxyMemberRef::Group { ref name } if name == "Nested"));
        assert!(matches!(members[1], ProxyMemberRef::Node { ref name, .. } if name == "core"));
        assert!(matches!(members[2], ProxyMemberRef::Node { ref name, .. } if name == "provider-only"));
        assert!(matches!(
            members[3],
            ProxyMemberRef::Unresolved {
                reason: ProxyMemberUnresolvedReason::Missing,
                ..
            }
        ));
        assert!(!view.groups.iter().any(|group| group.name == "GLOBAL"));
    }

    #[test]
    fn ambiguous_and_provider_unavailable_are_not_reported_as_missing() {
        let ready = ProxyViewBuilder::build(input_with_members_and_duplicate_providers());
        assert!(matches!(
            ready.groups[0].members[0],
            ProxyMemberRef::Unresolved {
                reason: ProxyMemberUnresolvedReason::Ambiguous,
                ..
            }
        ));

        let unavailable = ProxyViewBuilder::build(input_with_provider_member(None));
        assert!(matches!(
            unavailable.groups[0].members[0],
            ProxyMemberRef::Unresolved {
                reason: ProxyMemberUnresolvedReason::ProviderUnavailable,
                ..
            }
        ));
        assert_eq!(unavailable.provider_state, ProxyViewProviderState::Unavailable);
        assert!(unavailable.providers.is_empty());
    }

    #[test]
    fn providers_preserve_identity_duplicates_and_member_order() {
        let view = ProxyViewBuilder::build(input_with_provider_duplicates());
        assert_eq!(
            view.providers
                .iter()
                .map(|provider| provider.name.as_str())
                .collect::<Vec<_>>(),
            ["a-provider", "z-provider"]
        );
        assert_eq!(view.providers[0].proxy_record_ids, ["p:0:0", "p:0:1"]);
        assert_eq!(view.records["p:0:0"].name, "same");
        assert_eq!(view.records["p:0:1"].name, "same");
        assert_eq!(
            view.records["p:0:0"].source,
            ProxyNodeSource::Provider {
                provider_name: "a-provider".into(),
                proxy_name: "same".into(),
            }
        );
    }

    #[test]
    fn a_successful_response_with_only_unsupported_providers_is_ready_and_empty() {
        let view = ProxyViewBuilder::build(ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies::default(),
            providers: Some(providers_fixture(vec![(
                "unsupported",
                VehicleType::Compatible,
                &["ignored"],
            )])),
            delay_display: Default::default(),
        });

        assert_eq!(view.provider_state, ProxyViewProviderState::Ready);
        assert!(view.providers.is_empty());
    }

    #[test]
    fn display_percent_uses_profile_provider_and_group_sources() {
        let mut core_group = group("Core Group", &["core"]);
        core_group.now = Some("core".into());
        let mut provider_group = group("Provider Group", &["provider-node"]);
        provider_group.now = Some("provider-node".into());
        let proxies = HashMap::from([
            ("Core Group".to_owned(), core_group),
            ("Provider Group".to_owned(), provider_group),
            ("core".to_owned(), node("core")),
        ]);
        let runtime: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxy-providers:
  provider-a:
    url: https://cdn.example.com/slove/provider.yaml
"#,
        )
        .expect("parse runtime providers");
        let delay_display = DelayDisplayContext::from_runtime(Some("https://example.com/profile.yaml"), Some(&runtime));

        let view = ProxyViewBuilder::build(ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies { proxies },
            providers: Some(providers_fixture(vec![(
                "provider-a",
                VehicleType::HTTP,
                &["provider-node"],
            )])),
            delay_display,
        });

        assert_eq!(view.records["c:0"].delay_display_percent, DEFAULT_DELAY_DISPLAY_PERCENT);
        assert_eq!(view.records["p:0:0"].delay_display_percent, SLOVE_DELAY_DISPLAY_PERCENT);
        assert_eq!(
            view.groups
                .iter()
                .find(|group| group.name == "Core Group")
                .expect("core group")
                .delay_display_percent,
            DEFAULT_DELAY_DISPLAY_PERCENT
        );
        assert_eq!(
            view.groups
                .iter()
                .find(|group| group.name == "Provider Group")
                .expect("provider group")
                .delay_display_percent,
            SLOVE_DELAY_DISPLAY_PERCENT
        );

        let json = serde_json::to_value(view).expect("serialize view");
        assert_eq!(json["records"]["c:0"]["delayDisplayPercent"], 200);
        assert_eq!(json["records"]["p:0:0"]["delayDisplayPercent"], 40);
    }

    #[test]
    fn cyclic_group_display_percent_falls_back_to_default() {
        let mut alpha = group("Alpha", &["Beta"]);
        alpha.now = Some("Beta".into());
        let mut beta = group("Beta", &["Alpha"]);
        beta.now = Some("Alpha".into());
        let proxies = HashMap::from([("Alpha".to_owned(), alpha), ("Beta".to_owned(), beta)]);
        let delay_display = DelayDisplayContext::from_runtime(Some("https://example.com/slove/profile.yaml"), None);

        let view = ProxyViewBuilder::build(ProxyViewInput {
            runtime_group_order: vec![],
            proxies: Proxies { proxies },
            providers: None,
            delay_display,
        });

        for name in ["Alpha", "Beta"] {
            assert_eq!(
                view.groups
                    .iter()
                    .find(|group| group.name == name)
                    .expect("cyclic group")
                    .delay_display_percent,
                DEFAULT_DELAY_DISPLAY_PERCENT
            );
        }
    }

    #[test]
    fn serde_matches_the_v1_wire_contract() {
        let view = ProxyViewBuilder::build(serde_input_with_provider());
        let json = serde_json::to_value(view).expect("serialize view");

        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["orderSource"], "fallback");
        assert_eq!(json["providerState"], "ready");
        assert!(json["global"].is_null());
        assert!(json["direct"].is_null());
        assert!(json["providers"][0].get("vehicleType").is_some());
        assert!(json["providers"][0].get("updatedAt").is_none());
        assert_eq!(json["providers"][0]["subscriptionInfo"]["upload"], 1);
        assert!(json["providers"][0]["subscriptionInfo"].get("Upload").is_none());
    }
}
