use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// **Property 7: Installation Guide Completeness**
/// **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6**
/// 
/// This property-based test validates that the installation documentation is comprehensive
/// and covers all necessary aspects for successful Linux installation of Meetily.

#[derive(Debug, Clone)]
pub struct InstallationGuide {
    pub prerequisites: Vec<String>,
    pub ollama_integration: Vec<String>,
    pub build_instructions: Vec<String>,
    pub deployment_steps: Vec<String>,
    pub troubleshooting_sections: Vec<String>,
    pub configuration_examples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SystemRequirement {
    pub name: String,
    pub version: Option<String>,
    pub package_manager: String,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct InstallationStep {
    pub description: String,
    pub commands: Vec<String>,
    pub validation: Option<String>,
    pub error_handling: Option<String>,
}

impl InstallationGuide {
    pub fn from_documentation() -> Result<Self, Box<dyn std::error::Error>> {
        let linux_guide = fs::read_to_string("../docs/LINUX_INSTALLATION_GUIDE.md")?;
        let ollama_guide = fs::read_to_string("../docs/OLLAMA_INTEGRATION_GUIDE.md")?;
        let build_guide = fs::read_to_string("../docs/BUILD_AND_DEPLOYMENT_GUIDE.md")?;
        let troubleshooting_guide = fs::read_to_string("../docs/INSTALLATION_TROUBLESHOOTING.md")?;
        let config_examples = fs::read_to_string("../docs/CONFIGURATION_EXAMPLES.md")?;

        Ok(InstallationGuide {
            prerequisites: Self::extract_prerequisites(&linux_guide),
            ollama_integration: Self::extract_ollama_steps(&ollama_guide),
            build_instructions: Self::extract_build_steps(&build_guide),
            deployment_steps: Self::extract_deployment_steps(&build_guide),
            troubleshooting_sections: Self::extract_troubleshooting_sections(&troubleshooting_guide),
            configuration_examples: Self::extract_configuration_examples(&config_examples),
        })
    }

    fn extract_prerequisites(content: &str) -> Vec<String> {
        let mut prerequisites = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_prerequisites = false;

        for line in lines {
            if line.contains("## System Requirements") || line.contains("## Prerequisites") {
                in_prerequisites = true;
                continue;
            }
            if in_prerequisites && line.starts_with("## ") && !line.contains("Requirements") {
                break;
            }
            if in_prerequisites && (line.contains("sudo apt install") || line.contains("sudo dnf install") || line.contains("sudo pacman -S")) {
                prerequisites.push(line.trim().to_string());
            }
        }
        prerequisites
    }

    fn extract_ollama_steps(content: &str) -> Vec<String> {
        let mut steps = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_installation = false;

        for line in lines {
            if line.contains("## Installation") {
                in_installation = true;
                continue;
            }
            if in_installation && line.starts_with("## ") {
                break;
            }
            if in_installation && line.starts_with("```bash") {
                in_installation = false; // Start collecting commands
                continue;
            }
            if line.starts_with("```") && !in_installation {
                in_installation = true; // End of command block
                continue;
            }
            if !in_installation && !line.trim().is_empty() && !line.starts_with("#") {
                steps.push(line.trim().to_string());
            }
        }
        steps
    }

    fn extract_build_steps(content: &str) -> Vec<String> {
        let mut steps = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_build_section = false;

        for line in lines {
            if line.contains("## Building") || line.contains("## Development Setup") {
                in_build_section = true;
                continue;
            }
            if in_build_section && line.starts_with("## ") && !line.contains("Build") {
                break;
            }
            if in_build_section && (line.contains("cargo build") || line.contains("npm install") || line.contains("python -m venv")) {
                steps.push(line.trim().to_string());
            }
        }
        steps
    }

    fn extract_deployment_steps(content: &str) -> Vec<String> {
        let mut steps = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_deployment = false;

        for line in lines {
            if line.contains("## Production Deployment") || line.contains("## Docker Deployment") {
                in_deployment = true;
                continue;
            }
            if in_deployment && line.starts_with("## ") && !line.contains("Deployment") {
                break;
            }
            if in_deployment && (line.contains("docker") || line.contains("systemctl") || line.contains("nginx")) {
                steps.push(line.trim().to_string());
            }
        }
        steps
    }

    fn extract_troubleshooting_sections(content: &str) -> Vec<String> {
        let mut sections = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            if line.starts_with("## ") && !line.contains("Table of Contents") {
                sections.push(line.trim_start_matches("## ").to_string());
            }
        }
        sections
    }

    fn extract_configuration_examples(content: &str) -> Vec<String> {
        let mut examples = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            if line.starts_with("### ") {
                examples.push(line.trim_start_matches("### ").to_string());
            }
        }
        examples
    }
}

// Property test generators
prop_compose! {
    fn arb_system_requirement()
        (name in "[a-z]{3,15}",
         version in prop::option::of("[0-9]{1,2}\\.[0-9]{1,2}"),
         package_manager in prop::sample::select(vec!["apt".to_string(), "dnf".to_string(), "pacman".to_string(), "yum".to_string()]),
         is_optional in any::<bool>())
        -> SystemRequirement
    {
        SystemRequirement {
            name,
            version,
            package_manager,
            is_optional,
        }
    }
}

prop_compose! {
    fn arb_installation_step()
        (description in "[a-zA-Z ]{10,50}",
         commands in prop::collection::vec("[a-z ]{5,30}", 1..5),
         validation in prop::option::of("[a-z ]{5,20}"),
         error_handling in prop::option::of("[a-z ]{5,30}"))
        -> InstallationStep
    {
        InstallationStep {
            description,
            commands,
            validation,
            error_handling,
        }
    }
}

// Core property tests
proptest! {
    #[test]
    fn property_installation_guide_has_all_required_sections(
        _dummy in any::<u8>() // Dummy input since we're testing actual files
    ) {
        let guide = InstallationGuide::from_documentation()
            .expect("Should be able to read installation documentation");

        // Property 1: Prerequisites section must be comprehensive
        prop_assert!(
            !guide.prerequisites.is_empty(),
            "Installation guide must include system prerequisites"
        );

        // Verify essential prerequisites are covered
        let prereq_text = guide.prerequisites.join(" ").to_lowercase();
        let essential_deps = vec![
            "rust", "cargo", "node", "npm", "python", "git", 
            "build-essential", "cmake", "pkg-config", "libssl"
        ];
        
        for dep in essential_deps {
            prop_assert!(
                prereq_text.contains(dep) || 
                guide.prerequisites.iter().any(|p| p.to_lowercase().contains(dep)),
                "Prerequisites must include {}", dep
            );
        }

        // Property 2: Ollama integration must be documented
        prop_assert!(
            !guide.ollama_integration.is_empty(),
            "Installation guide must include Ollama integration steps"
        );

        let ollama_text = guide.ollama_integration.join(" ").to_lowercase();
        let ollama_requirements = vec!["curl", "install", "ollama", "pull", "serve"];
        
        for req in ollama_requirements {
            prop_assert!(
                ollama_text.contains(req),
                "Ollama integration must include {}", req
            );
        }

        // Property 3: Build instructions must be complete
        prop_assert!(
            !guide.build_instructions.is_empty(),
            "Installation guide must include build instructions"
        );

        let build_text = guide.build_instructions.join(" ").to_lowercase();
        let build_requirements = vec!["cargo build", "npm install", "venv"];
        
        for req in build_requirements {
            prop_assert!(
                build_text.contains(req),
                "Build instructions must include {}", req
            );
        }

        // Property 4: Deployment steps must be documented
        prop_assert!(
            !guide.deployment_steps.is_empty(),
            "Installation guide must include deployment steps"
        );

        // Property 5: Troubleshooting must be comprehensive
        prop_assert!(
            guide.troubleshooting_sections.len() >= 10,
            "Troubleshooting guide must have at least 10 sections, found {}",
            guide.troubleshooting_sections.len()
        );

        let troubleshooting_topics = vec![
            "system requirements", "dependency", "rust", "node", "python", 
            "ollama", "gpu", "audio", "database", "network", "build", "performance"
        ];
        
        let sections_text = guide.troubleshooting_sections.join(" ").to_lowercase();
        let covered_topics = troubleshooting_topics.iter()
            .filter(|topic| sections_text.contains(*topic))
            .count();
            
        prop_assert!(
            covered_topics >= 8,
            "Troubleshooting must cover at least 8 major topics, found {}",
            covered_topics
        );

        // Property 6: Configuration examples must be provided
        prop_assert!(
            guide.configuration_examples.len() >= 15,
            "Configuration examples must have at least 15 examples, found {}",
            guide.configuration_examples.len()
        );

        let config_topics = vec![
            "environment", "backend", "frontend", "ollama", "gpu", 
            "audio", "database", "security", "docker", "systemd"
        ];
        
        let config_text = guide.configuration_examples.join(" ").to_lowercase();
        let covered_configs = config_topics.iter()
            .filter(|topic| config_text.contains(*topic))
            .count();
            
        prop_assert!(
            covered_configs >= 8,
            "Configuration examples must cover at least 8 major areas, found {}",
            covered_configs
        );
    }

    #[test]
    fn property_installation_steps_are_executable(
        steps in prop::collection::vec(arb_installation_step(), 1..10)
    ) {
        // Property: All installation steps must be executable and have proper validation
        for step in steps {
            // Each step must have a clear description
            prop_assert!(
                step.description.len() >= 10,
                "Installation step description must be at least 10 characters"
            );

            // Each step must have at least one command
            prop_assert!(
                !step.commands.is_empty(),
                "Installation step must have at least one command"
            );

            // Commands should not be empty
            for command in &step.commands {
                prop_assert!(
                    !command.trim().is_empty(),
                    "Installation commands must not be empty"
                );
            }

            // If validation is provided, it should be meaningful
            if let Some(validation) = &step.validation {
                prop_assert!(
                    validation.len() >= 5,
                    "Validation steps must be at least 5 characters"
                );
            }

            // Error handling should be provided for critical steps
            if step.commands.iter().any(|cmd| 
                cmd.contains("sudo") || cmd.contains("install") || cmd.contains("build")
            ) {
                prop_assert!(
                    step.error_handling.is_some(),
                    "Critical installation steps must include error handling"
                );
            }
        }
    }

    #[test]
    fn property_system_requirements_are_comprehensive(
        requirements in prop::collection::vec(arb_system_requirement(), 5..20)
    ) {
        // Property: System requirements must cover all necessary components
        let mut package_managers = HashSet::new();
        let mut has_build_tools = false;
        let mut has_runtime_deps = false;
        let mut has_optional_deps = false;

        for req in requirements {
            package_managers.insert(req.package_manager.clone());
            
            if req.name.contains("build") || req.name.contains("cmake") || req.name.contains("gcc") {
                has_build_tools = true;
            }
            
            if req.name.contains("python") || req.name.contains("node") || req.name.contains("rust") {
                has_runtime_deps = true;
            }
            
            if req.is_optional {
                has_optional_deps = true;
            }
        }

        // Must support multiple package managers
        prop_assert!(
            package_managers.len() >= 2,
            "Installation guide must support at least 2 package managers"
        );

        // Must include build tools
        prop_assert!(has_build_tools, "System requirements must include build tools");

        // Must include runtime dependencies
        prop_assert!(has_runtime_deps, "System requirements must include runtime dependencies");

        // Should distinguish between required and optional dependencies
        prop_assert!(has_optional_deps, "System requirements should include optional dependencies");
    }

    #[test]
    fn property_documentation_cross_references_are_valid(
        _dummy in any::<u8>()
    ) {
        // Property: All cross-references between documentation files must be valid
        let docs_dir = Path::new("../docs");
        prop_assert!(docs_dir.exists(), "Documentation directory must exist");

        let required_files = vec![
            "LINUX_INSTALLATION_GUIDE.md",
            "OLLAMA_INTEGRATION_GUIDE.md", 
            "BUILD_AND_DEPLOYMENT_GUIDE.md",
            "INSTALLATION_TROUBLESHOOTING.md",
            "CONFIGURATION_EXAMPLES.md"
        ];

        for file in required_files {
            let file_path = docs_dir.join(file);
            prop_assert!(
                file_path.exists(),
                "Required documentation file {} must exist",
                file
            );

            let content = fs::read_to_string(&file_path)
                .expect("Should be able to read documentation file");
            
            // Check that file is not empty
            prop_assert!(
                content.len() > 100,
                "Documentation file {} must have substantial content",
                file
            );

            // Check for proper markdown structure
            prop_assert!(
                content.contains("# ") || content.contains("## "),
                "Documentation file {} must have proper markdown headers",
                file
            );
        }

        // Verify cross-references between files
        let linux_guide = fs::read_to_string(docs_dir.join("LINUX_INSTALLATION_GUIDE.md"))
            .expect("Should read Linux installation guide");
            
        // Linux guide should reference other guides
        prop_assert!(
            linux_guide.contains("OLLAMA_INTEGRATION_GUIDE") || 
            linux_guide.to_lowercase().contains("ollama integration"),
            "Linux installation guide must reference Ollama integration"
        );

        prop_assert!(
            linux_guide.contains("BUILD_AND_DEPLOYMENT_GUIDE") || 
            linux_guide.to_lowercase().contains("build") && linux_guide.to_lowercase().contains("deployment"),
            "Linux installation guide must reference build and deployment"
        );
    }

    #[test]
    fn property_installation_validation_scripts_exist(
        _dummy in any::<u8>()
    ) {
        // Property: Installation validation scripts must be provided and functional
        let troubleshooting_content = fs::read_to_string("../docs/INSTALLATION_TROUBLESHOOTING.md")
            .expect("Should be able to read troubleshooting guide");

        // Must include validation scripts
        prop_assert!(
            troubleshooting_content.contains("validate_system.sh") ||
            troubleshooting_content.contains("validation script"),
            "Troubleshooting guide must include system validation scripts"
        );

        prop_assert!(
            troubleshooting_content.contains("test_installation.sh") ||
            troubleshooting_content.contains("installation test"),
            "Troubleshooting guide must include installation test scripts"
        );

        // Scripts must check essential components
        let essential_checks = vec![
            "ram", "disk", "rustc", "cargo", "node", "npm", "python", "ollama"
        ];

        let content_lower = troubleshooting_content.to_lowercase();
        let covered_checks = essential_checks.iter()
            .filter(|check| content_lower.contains(*check))
            .count();

        prop_assert!(
            covered_checks >= 6,
            "Validation scripts must check at least 6 essential components, found {}",
            covered_checks
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_guide_parsing() {
        // Unit test to verify our parsing logic works correctly
        let sample_content = r#"
# Linux Installation Guide

## System Requirements

### Hardware Requirements
- RAM: 8GB minimum, 16GB recommended
- Storage: 20GB free space

### Software Dependencies

Install required packages:

```bash
sudo apt install -y build-essential cmake pkg-config libssl-dev
sudo apt install -y python3 python3-pip python3-venv
sudo apt install -y nodejs npm
```

## Ollama Integration

See OLLAMA_INTEGRATION_GUIDE.md for details.
"#;

        let prerequisites = InstallationGuide::extract_prerequisites(sample_content);
        assert!(!prerequisites.is_empty());
        assert!(prerequisites.iter().any(|p| p.contains("build-essential")));
        assert!(prerequisites.iter().any(|p| p.contains("python3")));
    }

    #[test]
    fn test_system_requirement_validation() {
        let req = SystemRequirement {
            name: "rust".to_string(),
            version: Some("1.70".to_string()),
            package_manager: "apt".to_string(),
            is_optional: false,
        };

        assert_eq!(req.name, "rust");
        assert!(!req.is_optional);
    }

    #[test]
    fn test_installation_step_validation() {
        let step = InstallationStep {
            description: "Install Rust toolchain".to_string(),
            commands: vec!["curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string()],
            validation: Some("rustc --version".to_string()),
            error_handling: Some("Check internet connection and try again".to_string()),
        };

        assert!(!step.commands.is_empty());
        assert!(step.validation.is_some());
        assert!(step.error_handling.is_some());
    }
}