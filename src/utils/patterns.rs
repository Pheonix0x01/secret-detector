use regex::Regex;
use lazy_static::lazy_static;
use crate::models::scan::Severity;

pub struct SecretPattern {
    pub name: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
}

lazy_static! {
    pub static ref SECRET_PATTERNS: Vec<SecretPattern> = vec![
        SecretPattern {
            name: "AWS Access Key ID".to_string(),
            pattern: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            severity: Severity::Critical,
            description: "AWS Access Key ID detected".to_string(),
            remediation: "Immediately rotate this key in AWS IAM console and revoke the exposed key".to_string(),
        },
        SecretPattern {
            name: "AWS Secret Access Key".to_string(),
            pattern: Regex::new(r#"aws_secret_access_key\s*=\s*['"]?[A-Za-z0-9/+=]{40}['"]?"#).unwrap(),
            severity: Severity::Critical,
            description: "AWS Secret Access Key detected".to_string(),
            remediation: "Rotate the corresponding AWS access key immediately".to_string(),
        },
        SecretPattern {
            name: "OpenAI API Key".to_string(),
            pattern: Regex::new(r"sk-[A-Za-z0-9]{48}").unwrap(),
            severity: Severity::Critical,
            description: "OpenAI API key detected".to_string(),
            remediation: "Revoke this key in OpenAI dashboard and generate a new one".to_string(),
        },
        SecretPattern {
            name: "Stripe API Key".to_string(),
            pattern: Regex::new(r"sk_live_[0-9a-zA-Z]{24}").unwrap(),
            severity: Severity::Critical,
            description: "Stripe live API key detected".to_string(),
            remediation: "Immediately revoke this key in Stripe dashboard".to_string(),
        },
        SecretPattern {
            name: "SendGrid API Key".to_string(),
            pattern: Regex::new(r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}").unwrap(),
            severity: Severity::High,
            description: "SendGrid API key detected".to_string(),
            remediation: "Revoke this key in SendGrid settings".to_string(),
        },
        SecretPattern {
            name: "Generic API Key".to_string(),
            pattern: Regex::new(r#"(?i)api[_-]?key\s*[:=]\s*['"]?[A-Za-z0-9_\-]{20,}['"]?"#).unwrap(),
            severity: Severity::High,
            description: "Possible API key detected".to_string(),
            remediation: "Verify if this is a real API key and rotate if needed".to_string(),
        },
        SecretPattern {
            name: "Database Connection String".to_string(),
            pattern: Regex::new(r"(mysql|postgresql|mongodb)://[^:]+:[^@]+@").unwrap(),
            severity: Severity::Critical,
            description: "Database connection string with credentials detected".to_string(),
            remediation: "Change database password and use environment variables".to_string(),
        },
        SecretPattern {
            name: "Password in Code".to_string(),
            pattern: Regex::new(r#"(?i)password\s*[:=]\s*['"][^'"]{8,}['"]"#).unwrap(),
            severity: Severity::High,
            description: "Hardcoded password detected".to_string(),
            remediation: "Remove password from code and use secure configuration".to_string(),
        },
        SecretPattern {
            name: "OAuth Token".to_string(),
            pattern: Regex::new(r#"(?i)oauth[_-]?token\s*[:=]\s*['"]?[A-Za-z0-9_\-]{20,}['"]?"#).unwrap(),
            severity: Severity::High,
            description: "OAuth token detected".to_string(),
            remediation: "Revoke this token and regenerate".to_string(),
        },
        SecretPattern {
            name: "JWT Token".to_string(),
            pattern: Regex::new(r"eyJ[A-Za-z0-9_-]*\.eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*").unwrap(),
            severity: Severity::Medium,
            description: "JWT token detected".to_string(),
            remediation: "Ensure this token is expired or revoke and regenerate".to_string(),
        },
        SecretPattern {
            name: "RSA Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN (RSA )?PRIVATE KEY-----").unwrap(),
            severity: Severity::Critical,
            description: "RSA private key detected".to_string(),
            remediation: "Remove private key from repository and regenerate key pair".to_string(),
        },
        SecretPattern {
            name: "SSH Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN OPENSSH PRIVATE KEY-----").unwrap(),
            severity: Severity::Critical,
            description: "SSH private key detected".to_string(),
            remediation: "Remove SSH key from repository and regenerate".to_string(),
        },
    ];
}

pub fn should_scan_file(path: &str) -> bool {
    let skip_extensions = vec![
        ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico",
        ".pdf", ".zip", ".tar", ".gz", ".exe", ".dll",
        ".so", ".dylib", ".bin", ".dat", ".lock"
    ];
    
    let skip_dirs = vec![
        "node_modules", "vendor", "dist", "build", ".git",
        "target", "venv", "__pycache__", ".next"
    ];

    for ext in skip_extensions {
        if path.ends_with(ext) {
            return false;
        }
    }

    for dir in skip_dirs {
        if path.contains(&format!("/{}/", dir)) || path.starts_with(&format!("{}/", dir)) {
            return false;
        }
    }

    true
}

pub fn is_likely_test_or_example(path: &str) -> bool {
    let test_indicators = vec![
        "test", "tests", "spec", "example", "examples",
        "sample", "samples", "mock", "fixture", "demo"
    ];

    let path_lower = path.to_lowercase();
    
    for indicator in test_indicators {
        if path_lower.contains(indicator) {
            return true;
        }
    }

    false
}