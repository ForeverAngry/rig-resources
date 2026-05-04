//! Security-oriented skill catalog.

pub mod credential;
pub mod exfil;
pub mod lateral;
pub mod recon;

use std::sync::Arc;

use rig_compose::{Skill, SkillRegistry};

/// Register every security skill in the catalog with `registry`. Returns
/// the same registry for fluent use.
pub fn register_default_catalog(registry: &SkillRegistry) -> &SkillRegistry {
    let skills: Vec<Arc<dyn Skill>> = vec![
        Arc::new(recon::HighFanoutSkill::default()),
        Arc::new(recon::EntropyCheckSkill::default()),
        Arc::new(lateral::AuthSpawnConnectSkill),
        Arc::new(credential::PasswordSpraySkill),
        Arc::new(exfil::SlowBeaconSkill),
        Arc::new(crate::skills::BaselineCompareSkill),
        Arc::new(crate::skills::MemoryPivotSkill::default()),
    ];
    for skill in skills {
        registry.register(skill);
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_registers_all_skills() {
        let registry = SkillRegistry::new();
        register_default_catalog(&registry);
        for id in [
            "recon.high_fanout",
            "recon.entropy_check",
            "lateral.auth_spawn_connect",
            "credential.password_spray",
            "exfil.slow_beacon",
            "general.baseline_compare",
            "general.memory_pivot",
        ] {
            assert!(registry.get(id).is_ok(), "missing {id}");
        }
    }
}
