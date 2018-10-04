use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct RPM {
    pub name: String,
    version: String,
    release: String,
    arch: RpmArch,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum RpmArch {
    Noarch,
    X86_64,
    Src,
}

fn get_nvra(filename: &str) -> Option<(&str, &str, &str, &str)> {
    let i = filename.rfind(".rpm")?;
    let n = &filename[..i];

    let arch_idx = n.rfind(".")?;
    let arch = &n[arch_idx + 1..];

    let release_idx = n[..arch_idx].rfind("-")?;
    let release = &n[release_idx + 1..arch_idx];

    let version_idx = n[..release_idx].rfind("-")?;
    let version = &n[version_idx + 1..release_idx];

    let name = &n[..version_idx];
    Some((name, version, release, arch))
}

fn get_rpm_dependencies(filepath: &PathBuf) -> Vec<String> {
    let output = Command::new("rpm")
        .arg("-qRp")
        .arg(filepath)
        .output()
        .expect("Failed to run command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    rpm_output_to_deps(&stdout)
}

fn rpm_output_to_deps(output: &str) -> Vec<String> {
    let mut rpms: Vec<String> = vec![];
    output.split('\n').for_each(|l| {
        if !l.starts_with("rpmlib") {
            let s_l: Vec<&str> = l.split(' ').collect();
            rpms.push(s_l[0].to_string());
        }
    });
    rpms
}

impl RPM {
    // Creates a new RPM instance from the filename
    pub fn new(filepath: &PathBuf) -> Option<RPM> {
        if !filepath.exists() {
            return None;
        }

        let name = filepath.file_name()?;
        let (n, v, r, a) = get_nvra(name.to_str().unwrap())?;
        let arch = match a {
            "noarch" => RpmArch::Noarch,
            "x86_64" => RpmArch::X86_64,
            "src" => RpmArch::Src,
            _ => unreachable!(),
        };

        let deps = get_rpm_dependencies(filepath);

        Some(RPM {
            name: n.to_string(),
            version: v.to_string(),
            release: r.to_string(),
            arch: arch,
            deps: deps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::{remove_file, File};

    #[test]
    fn get_nvra_from_filename() {
        let f = "acl-2.2.51-12.el7.x86_64.rpm";
        if let Some((n, v, r, a)) = get_nvra(f) {
            assert_eq!(n, "acl");
            assert_eq!(a, "x86_64");
            assert_eq!(v, "2.2.51");
            assert_eq!(r, "12.el7");
        }

        let f = "acl-2.2.51-12.el7.x86_64";
        assert_eq!(None, get_nvra(f));

        let f = "acl-2.2.51.12.el7.x86_64.rpm";
        assert_eq!(None, get_nvra(f));

        let f = "some-other-invalid-string.rpm";
        assert_eq!(None, get_nvra(f));
    }

    #[test]
    fn get_rpm_from_filename() {
        // Create mock file
        let name = "firewalld-0.4.4.4-14.el7.noarch.rpm";

        let _ = File::create(name);
        let name = PathBuf::from(name);
        let rpm = RPM::new(&name).unwrap();
        assert_eq!(rpm.name, "firewalld");
        assert_eq!(rpm.version, "0.4.4.4");
        assert_eq!(rpm.release, "14.el7");
        assert_eq!(rpm.arch, RpmArch::Noarch);

        remove_file(name).unwrap();
    }

    #[test]
    fn get_wrong_rpm_from_filename() {
        let name = PathBuf::from("firewalld-0.4.4.4-14.el7.noarch");
        let rpm = RPM::new(&name);
        assert_eq!(None, rpm);
    }

    #[test]
    fn rpm_output_to_deps_vec() {
        let output = "lua-devel >= 5.1
libcap-devel
libacl-devel
xz-devel >= 4.999.8
dbus-devel
lua-devel
nspr-devel
rpmlib(FileDigests) <= 4.6.0-1
rpmlib(CompressedFileNames) <= 3.0.4-1";

        let deps = rpm_output_to_deps(output);
        assert_eq!(7, deps.len());

        let output = "lua-devel >= 5.1";
        let deps = rpm_output_to_deps(output);
        assert_eq!("lua-devel", deps[0]);
    }
}
