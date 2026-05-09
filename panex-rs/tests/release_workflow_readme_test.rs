const RELEASE_WORKFLOW: &str = include_str!("../../.github/workflows/release.yml");

fn create_npm_package_step() -> &'static str {
    let start = RELEASE_WORKFLOW
        .find("- name: Create npm package")
        .expect("release workflow should create platform npm packages");
    let end = RELEASE_WORKFLOW[start..]
        .find("- name: Upload artifact")
        .map(|offset| start + offset)
        .expect("release workflow should upload platform package artifacts");

    &RELEASE_WORKFLOW[start..end]
}

fn generated_readme_content() -> &'static str {
    let step = create_npm_package_step();
    let marker = "cat > npm-pkg/README.md <<";
    let heredoc = step
        .find(marker)
        .map(|offset| &step[offset + marker.len()..])
        .expect("Create npm package step should write npm-pkg/README.md");
    let body_start = heredoc
        .find('\n')
        .expect("generated README heredoc should include content");
    let readme_body = &heredoc[body_start..];
    let end = readme_body
        .find("\n          EOF")
        .or_else(|| readme_body.find("\nEOF"))
        .expect("generated README heredoc should be closed with EOF");

    readme_body[..end].trim()
}

#[test]
fn release_workflow_readme_test_create_step_writes_readme_before_artifact_upload() {
    let step_start = RELEASE_WORKFLOW
        .find("- name: Create npm package")
        .expect("release workflow should create platform npm packages");
    let readme_write = RELEASE_WORKFLOW
        .find("npm-pkg/README.md")
        .expect("Create npm package step should write npm-pkg/README.md");
    let upload_artifact = RELEASE_WORKFLOW
        .find("- name: Upload artifact")
        .expect("release workflow should upload platform package artifacts");

    assert!(step_start < readme_write);
    assert!(readme_write < upload_artifact);
}

#[test]
fn release_workflow_readme_test_generated_readme_includes_short_panex_description() {
    assert!(generated_readme_content()
        .contains("Terminal UI for running multiple processes in parallel"));
}

#[test]
fn release_workflow_readme_test_generated_readme_links_to_main_npm_package() {
    assert!(generated_readme_content().contains("https://www.npmjs.com/package/panex"));
}

#[test]
fn release_workflow_readme_test_generated_readme_links_to_repository() {
    assert!(generated_readme_content().contains("https://github.com/king8fisher/panex"));
}

#[test]
fn release_workflow_readme_test_generated_readme_explains_platform_package_installation_path() {
    let readme = generated_readme_content();

    assert!(readme.contains("Platform-specific Panex binary"));
    assert!(readme.contains("optional dependency of the main `panex` package"));
    assert!(readme.contains("Install and use `panex` directly"));
}
