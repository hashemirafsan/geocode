use serde::{Deserialize, Serialize};

use crate::runtime::{HostDiscovery, KnownBinary};

use super::{
    catalog::{capability_catalog, BindingFamilyKind, CapabilityStatus, CatalogCapability},
    types::{
        BindingKind, CapabilityBinding, CapabilityDescriptor, CapabilityId,
        PlannerCapabilityDescriptor,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRegistry {
    pub discovery: HostDiscovery,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistry {
    pub fn discover() -> Self {
        let discovery = HostDiscovery::discover();
        let catalog = capability_catalog();
        let capabilities = catalog
            .capabilities
            .iter()
            .filter(|entry| entry.status == CapabilityStatus::Implemented)
            .filter_map(|entry| descriptor_from_catalog(entry, &catalog, &discovery))
            .collect();

        Self {
            discovery,
            capabilities,
        }
    }

    pub fn descriptor(&self, id: CapabilityId) -> Option<&CapabilityDescriptor> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
    }

    pub fn planner_surface(&self) -> Vec<PlannerCapabilityDescriptor> {
        self.capabilities
            .iter()
            .map(|capability| PlannerCapabilityDescriptor {
                id: capability.id,
                summary: capability.summary.clone(),
                kind: capability.kind,
                input_type: capability.input_type.clone(),
                output_type: capability.output_type.clone(),
            })
            .collect()
    }
}

fn descriptor_from_catalog(
    entry: &CatalogCapability,
    catalog: &super::catalog::CapabilityCatalog,
    discovery: &HostDiscovery,
) -> Option<CapabilityDescriptor> {
    let id = CapabilityId::parse(entry.id)?;
    let bindings = entry
        .backends
        .iter()
        .filter(|route| route_available(route.backend, discovery))
        .filter_map(|route| {
            let family = catalog.binding_family(route.backend)?;
            let requirement = if !route.requires.is_empty() {
                Some(route.requires.join(", "))
            } else {
                route.notes.map(ToString::to_string)
            };
            Some(CapabilityBinding {
                kind: match family.kind {
                    BindingFamilyKind::LocalRuntime => BindingKind::LocalRuntime,
                    BindingFamilyKind::RustCrate => BindingKind::RustCrate,
                    BindingFamilyKind::KnownBinary => BindingKind::KnownBinary,
                },
                target: format!("{}::{}", route.backend, route.binding_op),
                requirement,
            })
        })
        .collect::<Vec<_>>();

    if bindings.is_empty() {
        return None;
    }

    Some(CapabilityDescriptor {
        id,
        label: entry.id.to_string(),
        summary: entry.summary.to_string(),
        kind: entry.kind,
        input_type: entry.input_type.to_string(),
        output_type: entry.output_type.to_string(),
        bindings,
    })
}

fn route_available(backend: &str, discovery: &HostDiscovery) -> bool {
    match backend {
        "ncdump_binary" => discovery.is_available(KnownBinary::NcDump),
        "gdalinfo_binary" => discovery.is_available(KnownBinary::GdalInfo),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use crate::capability::{capability_catalog, CapabilityId, CapabilityRegistry};

    #[test]
    fn planner_surface_contains_dataset_resolve() {
        let registry = CapabilityRegistry::discover();
        assert!(registry
            .planner_surface()
            .iter()
            .any(|capability| capability.id == CapabilityId::DatasetResolve));
    }

    #[test]
    fn rust_catalog_contains_planned_capability_inventory() {
        let catalog = capability_catalog();
        assert!(catalog
            .capabilities
            .iter()
            .any(|capability| capability.id == "netcdf.variable.load"));
        assert!(catalog
            .capabilities
            .iter()
            .any(|capability| capability.id == "raster.band.stats"));
    }
}
