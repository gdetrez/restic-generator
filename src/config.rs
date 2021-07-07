use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub source: String,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub repositories: Vec<RepositoryConfig>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepositoryConfig {
    pub name: String,
    pub location: String,
    pub password_command: Option<String>,
    pub password_file: Option<String>,
    pub aws_access_key: Option<String>,
    pub aws_secret_access_key: Option<String>,

    // Forget policies
    pub keep_last: Option<usize>,
    pub keep_hourly: Option<usize>,
    pub keep_daily: Option<usize>,
    pub keep_weekly: Option<usize>,
    pub keep_monthly: Option<usize>,
    pub keep_yearly: Option<usize>,
    pub keep_tag: Option<String>,
    pub keep_within: Option<String>,
}

impl RepositoryConfig {
    pub fn has_forget_policy(&self) -> bool {
        self.keep_last.is_some()
            || self.keep_hourly.is_some()
            || self.keep_daily.is_some()
            || self.keep_weekly.is_some()
            || self.keep_monthly.is_some()
            || self.keep_yearly.is_some()
            || self.keep_tag.is_some()
            || self.keep_within.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_forget_policy() {
        let repo = RepositoryConfig::default();
        assert!(!repo.has_forget_policy());
    }

    macro_rules! test_has_forget_policy {
        ( $testname:ident, $($attr:ident : $value:expr),+) => {
            #[test]
            fn $testname() {
                let repo = RepositoryConfig {
                    $($attr: Some($value)),+,
                    ..RepositoryConfig::default()
                };
                assert!(repo.has_forget_policy());
            }
        };
    }

    test_has_forget_policy!(keep_last_has_forget_policy, keep_last: 42);
    test_has_forget_policy!(keep_hourly_has_forget_policy, keep_hourly: 42);
    test_has_forget_policy!(keep_daily_has_forget_policy, keep_daily: 42);
    test_has_forget_policy!(keep_weekly_has_forget_policy, keep_weekly: 42);
    test_has_forget_policy!(keep_monthly_has_forget_policy, keep_monthly: 42);
    test_has_forget_policy!(keep_yearly_has_forget_policy, keep_yearly: 42);
    test_has_forget_policy!(keep_tag_has_forget_policy, keep_tag: "important".into());
    test_has_forget_policy!(keep_within_has_forget_policy, keep_within: "2y5m7d3h".into());
}
