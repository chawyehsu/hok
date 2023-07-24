#![allow(dead_code)]

use crate::{
    error::Fallible,
    internal::dag::DepGraph,
    package::{query, QueryOption},
    Error, Session,
};

use super::Package;

pub(crate) fn resolve_dependencies(session: &Session, packages: &mut Vec<Package>) -> Fallible<()> {
    let mut graph = DepGraph::<String>::new();
    let mut to_resolve = packages.clone();
    let options = vec![QueryOption::Explicit];

    loop {
        if to_resolve.is_empty() {
            break;
        }

        let tmp = to_resolve.clone();
        to_resolve = vec![];

        for pkg in tmp.into_iter() {
            let deps = pkg.dependencies();
            let mut dpkgs = vec![];

            if deps.is_empty() {
                graph.register_node(pkg.name().to_owned());
            } else {
                let queries = deps.iter().map(|d| d.as_str());

                for query in queries {
                    let dpkg = query::query_synced(session, query, &options)?;
                    if dpkg.is_empty() {
                        return Err(Error::PackageNotFound(query.to_owned()));
                    }
                    if dpkg.len() > 1 {
                        // TODO: handle this case smartly
                        return Err(Error::Custom(format!(
                            "Found multiple candidates for package named '{}'",
                            query
                        )));
                    }

                    if !packages.contains(&dpkg[0]) {
                        dpkgs.push(dpkg[0].clone());
                    }
                }

                let dep_nodes = dpkgs
                    .iter()
                    .map(|p: &Package| p.name().to_owned())
                    .collect::<Vec<_>>();
                graph.register_deps(pkg.name().to_owned(), dep_nodes);
            }
            // Cyclic dependency check
            graph.check()?;

            dpkgs.dedup();
            to_resolve.append(&mut dpkgs);
        }

        packages.extend(to_resolve.clone());
    }

    packages.dedup();
    packages.reverse();

    Ok(())
}

pub(crate) fn resolve_dependents(session: &Session, packages: &mut Vec<Package>) -> Fallible<()> {
    let mut to_resolve = packages.clone();
    let installed = query::query_installed(session, "*", &[QueryOption::Explicit])?;
    loop {
        if to_resolve.is_empty() {
            break;
        }

        let tmp = to_resolve.clone();
        to_resolve = vec![];

        for pkg in tmp.iter() {
            installed.iter().for_each(|p| {
                let be_dependent = p.dependencies().iter().any(|d| d == pkg.name());
                if be_dependent && !packages.contains(p) && !to_resolve.contains(p) {
                    to_resolve.push(p.clone());
                }
            })
        }

        packages.extend(to_resolve.clone());
    }

    packages.reverse();

    Ok(())
}
