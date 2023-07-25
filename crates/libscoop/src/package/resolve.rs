use crate::{
    error::Fallible,
    event,
    internal::dag::DepGraph,
    package::{query, Package, QueryOption},
    Error, Session,
};

/// Resolve dependencies of the given packages.
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
                    let mut candidates = query::query_synced(session, query, &options)?;
                    if candidates.is_empty() {
                        return Err(Error::PackageNotFound(query.to_owned()));
                    }
                    if candidates.len() > 1 {
                        select_candidate(session, &mut candidates)?;
                    }

                    if !packages.contains(&candidates[0]) {
                        dpkgs.push(candidates[0].clone());
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

/// Resolve packages that depend on the given packages.
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

/// Select one from multiple package candidates, interactively if possible.
pub(crate) fn select_candidate(session: &Session, candidates: &mut Vec<Package>) -> Fallible<()> {
    // Try to filter out installed ones if possible
    candidates.retain(|p| !p.is_strictly_installed());

    // Luckily, there is no more than one package left
    if candidates.len() <= 1 {
        return Ok(());
    }

    let name = candidates[0].name().to_owned();

    // Sort candidates by package ident, in other words, by alphabetical order
    // of bucket name.
    candidates.sort_by_key(|p| p.ident());

    // Only we can ask user/frontend to select one from multiple candidates
    // when the outbound tx is available for us to do an interactive q&a.
    if let Some(tx) = session.emitter() {
        let question = candidates.iter().map(|p| p.ident()).collect::<Vec<_>>();

        if tx.send(event::Event::SelectPackage(question)).is_ok() {
            // The unwrap is safe here because we have obtained the outbound tx,
            // so the inbound rx must be available.
            let rx = session.receiver().unwrap();

            while let Ok(answer) = rx.recv() {
                if let event::Event::SelectPackageAnswer(idx) = answer {
                    // bounds check
                    if idx < candidates.len() {
                        *candidates = vec![candidates[idx].clone()];

                        return Ok(());
                    }

                    return Err(Error::InvalidAnswer);
                }
            }
        }
    }

    // TODO: handle this case smartly using pre-defined bucket priority
    Err(Error::PackageMultipleCandidates(name))
}
