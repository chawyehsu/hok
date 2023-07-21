#![allow(dead_code)]
use log::debug;
use std::collections::HashSet;

use crate::{
    constant::REGEX_ARCHIVE_7Z,
    error::Fallible,
    internal::{dag::DepGraph, is_program_available},
    package::{query, QueryOption},
    Session,
};

use super::Package;

/// Resolve packages, including their dependencies, from given queries
pub fn resolve_packages(session: &Session, queries: HashSet<&str>) -> Fallible<Vec<Package>> {
    let mode = QueryOption::Explicit;
    let mut ret = query::query_synced(session, queries, vec![])?;
    let mut graph = DepGraph::<String>::new();
    let mut dep_pkgs = Vec::new();

    for pkg in ret.iter() {
        dep_pkgs.extend(visit_dependencies(&mut graph, pkg, session, mode)?);
    }
    ret.extend(dep_pkgs);

    let order = graph.walk_flatten()?;
    // Sort packages by package ident
    ret.sort_by(|a, b| {
        let a_idx = order.iter().position(|x| x == &a.ident()).unwrap();
        let b_idx = order.iter().position(|x| x == &b.ident()).unwrap();
        a_idx.cmp(&b_idx)
    });
    debug!("ordered dep graph {:?}", order);

    Ok(ret)
}

/// Recursively visit dependencies of a package and do cyclic dependencies check
fn visit_dependencies(
    graph: &mut DepGraph<String>,
    pkg: &Package,
    session: &Session,
    mode: QueryOption,
) -> Fallible<Vec<Package>> {
    let mut ret = Vec::new();

    match &pkg.dependencies {
        None => graph.register_node(pkg.ident()),
        Some(deps) => {
            let queries = deps.iter().map(|d| d.as_str()).collect();
            ret = query::query_synced(session, queries, vec![])?;

            // Cyclic dependencies check
            let dep_nodes = ret.iter().map(|p| p.ident()).collect::<Vec<_>>();
            graph.register_deps(pkg.ident(), dep_nodes);
            graph.check()?;

            let mut dep_pkgs = Vec::new();
            for pkg in ret.iter() {
                dep_pkgs.extend(visit_dependencies(graph, pkg, session, mode)?);
            }
            ret.extend(dep_pkgs);
        }
    }
    Ok(ret)
}

fn visit_external_dependeincies(package: &Package) -> Fallible<HashSet<String>> {
    let mut deps = HashSet::new();

    let url = package.manifest.url();
    let pre_install = package.manifest.pre_install().unwrap_or_default();
    let installer_script = package
        .manifest
        .installer()
        .map(|i| i.script().unwrap_or_default())
        .unwrap_or_default();
    let post_install = package.manifest.post_install().unwrap_or_default();
    let scripts = [pre_install, installer_script, post_install];

    // main/7zip
    if !is_program_available("7z.exe") {
        let archive_7z = url.iter().any(|u| REGEX_ARCHIVE_7Z.is_match(u));
        let script_7z = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-7zipArchive")));
        if archive_7z || script_7z {
            deps.insert("main/7zip".to_owned());
        }
    }

    // main/dark
    if !is_program_available("dark.exe") {
        let script_dark = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-DarkArchive")));
        if script_dark {
            deps.insert("main/dark".to_owned());
        }
    }

    // main/lessmsi
    if !is_program_available("lessmsi.exe") {
        let archive_msi = url.iter().any(|u| u.ends_with(".msi"));
        let script_msi = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-MsiArchive")));
        if archive_msi || script_msi {
            deps.insert("main/lessmsi".to_owned());
        }
    }

    // main/innounp
    if !is_program_available("innounp.exe") {
        let explicit_innounp = package.manifest.innosetup();
        let script_innounp = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-InnoArchive")));
        if explicit_innounp || script_innounp {
            deps.insert("main/innounp".to_owned());
        }
    }

    // main/zstd
    if !is_program_available("zstd.exe") {
        let archive_msi = url.iter().any(|u| u.ends_with(".zst"));
        let script_msi = scripts
            .iter()
            .any(|b| b.iter().any(|s| s.contains("Expand-ZstdArchive")));
        if archive_msi || script_msi {
            deps.insert("main/zstd".to_owned());
        }
    }

    Ok(deps)
}
