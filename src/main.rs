// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL-1.2 OR GPL-3.0
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

#![deny(warnings, clippy::all, clippy::pedantic,
    // Do cfg(test) right
    clippy::cfg_not_test,
    clippy::tests_outside_test_module,
    // Guard against left-over debugging output
    clippy::dbg_macro,
    clippy::unimplemented,
    clippy::use_debug,
    clippy::todo,
    // Don't panic carelessly
    clippy::get_unwrap,
    clippy::unused_result_ok,
    clippy::unwrap_in_result,
    clippy::indexing_slicing,
    // Do not carelessly ignore errors
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    // Code smells
    clippy::float_cmp_const,
    clippy::string_to_string,
    clippy::if_then_some_else_none,
    clippy::large_include_file,
    // Disable as casts
    clippy::as_conversions,
)]
#![forbid(unsafe_code)]

use std::io::prelude::*;

use alpm::{Alpm, Package};
use clap::Parser;
use pacgraph::graph::{DependencyEdge, PackageNode};
use petgraph::visit::{
    Data, EdgeFiltered, EdgeRef, GraphProp, GraphRef, IntoEdgeReferences, IntoNeighbors,
    IntoNeighborsDirected, IntoNodeIdentifiers, IntoNodeReferences, NodeCount, NodeIndexable,
    Reversed, Visitable, depth_first_search,
};
use tracing::debug;

use crate::{
    args::CliArgs,
    print::{DisplayPackageAnsi, print_package_graph},
};

mod args;
mod print;

fn list_orphans<'a, G>(options: &args::Orphans, graph: G) -> std::io::Result<()>
where
    G: GraphRef
        + GraphProp
        + Data<EdgeWeight = DependencyEdge, NodeWeight = PackageNode<'a>>
        + NodeCount
        + NodeIndexable
        + Visitable<NodeId = PackageNode<'a>>
        + IntoNeighbors
        + IntoNodeIdentifiers
        + IntoEdgeReferences
        + IntoNodeReferences,
{
    let orphans = pacgraph::dependencies::orphans(&graph);
    let with_version = !options.graph_options.quiet;
    let mut stdout = anstream::stdout().lock();
    if options.graph_options.dot {
        print_package_graph(&mut stdout, graph, with_version)
    } else {
        let mut orphan_nodes = orphans
            .node_identifiers()
            .map(PackageNode::package)
            .collect::<Vec<_>>();
        // Sort alphabetically
        orphan_nodes.sort_by_key(|pkg| pkg.name());

        for pkg in orphan_nodes {
            writeln!(
                &mut stdout,
                "{}",
                DisplayPackageAnsi::new(pkg).with_version(with_version)
            )?;
        }
        Ok(())
    }
}

fn orphans_command(options: &args::Orphans, alpm: &Alpm) -> std::io::Result<()> {
    let localdb = alpm.localdb();
    let pkg_graph = pacgraph::graph::build_graph_for_localdb(localdb);
    if options.graph_options.ignore_optdepends {
        list_orphans(
            options,
            &EdgeFiltered::from_fn(&pkg_graph, |edge| {
                *edge.weight() == DependencyEdge::Required
            }),
        )
    } else {
        list_orphans(options, &pkg_graph)
    }
}

fn list_dependents<'a, G>(
    options: &args::Dependents,
    pkg_graph: G,
    package: &'a Package,
) -> std::io::Result<()>
where
    G: GraphRef
        + GraphProp
        + NodeCount
        + Data<EdgeWeight = DependencyEdge, NodeWeight = PackageNode<'a>>
        + Visitable<NodeId = PackageNode<'a>>
        + NodeIndexable
        + IntoNeighborsDirected
        + IntoNodeIdentifiers
        + IntoNodeReferences
        + IntoEdgeReferences,
{
    let mut stdout = anstream::stdout().lock();
    let dependents = pacgraph::dependencies::dependents(&pkg_graph, package);
    let with_version = !options.graph_options.quiet;
    if options.graph_options.dot {
        print_package_graph(&mut stdout, &dependents, with_version)
    } else {
        let rdepends = Reversed(&dependents);
        let mut subtrees = HashMap::new();
        depth_first_search(
            &rdepends,
            [PackageNode::new(package)],
            |event| match event {
                petgraph::visit::DfsEvent::Discover(node, _) => {
                    debug!("Discover: {node}");
                }
                petgraph::visit::DfsEvent::TreeEdge(parent, child) => {
                    debug!("Edge: {parent} -> {child}");
                }
                petgraph::visit::DfsEvent::BackEdge(child, parent) => {
                    debug!("Back edge: {child} -> {parent}");
                }
                petgraph::visit::DfsEvent::CrossForwardEdge(a, b) => {
                    debug!("Forward edge: {a} -> {b}");
                }
                petgraph::visit::DfsEvent::Finish(node, _) => {
                    debug!("Finish: {node}");
                }
            },
        );
        Ok(())
        // for node in DfsPostOrder::new(&rdepends, PackageNode::new(package)).iter(&rdepends) {
        //     let tree = termtree::Tree::new(DisplayPackageAnsi::new(node.package())).with_leaves(
        //         rdepends
        //             .neighbors_directed(node, Direction::Outgoing)
        //             .map(|n| subtrees[&n].clone()),
        //     );
        //     subtrees.insert(node, tree);
        // }
        // writeln!(&mut stdout, "{}", subtrees[&PackageNode::new(package)])
    }
}

fn dependents_command(options: &args::Dependents, alpm: &Alpm) -> std::io::Result<()> {
    let localdb = alpm.localdb();
    let source_pkg = localdb
        .pkg(options.package.as_str())
        .map_err(std::io::Error::other)?;
    let pkg_graph = pacgraph::graph::build_graph_for_localdb(localdb);

    if options.graph_options.ignore_optdepends {
        list_dependents(
            options,
            &EdgeFiltered::from_fn(&pkg_graph, |edge| {
                *edge.weight() == DependencyEdge::Required
            }),
            source_pkg,
        )
    } else {
        list_dependents(options, &pkg_graph, source_pkg)
    }
}

fn main() -> std::io::Result<()> {
    use alpm_utils::{alpm_with_conf, config::Config};

    tracing_subscriber::fmt::init();

    let args = CliArgs::parse();

    let config = Config::new().map_err(|error| match error.kind {
        alpm_utils::config::ErrorKind::Io(error) => error,
        _ => std::io::Error::new(std::io::ErrorKind::InvalidData, error),
    })?;
    let alpm = alpm_with_conf(&config).map_err(std::io::Error::other)?;
    alpm.set_log_cb((), pacgraph::alpm::tracing_log_cb);

    match args.command {
        args::Command::Orphans(orphans) => orphans_command(&orphans, &alpm)?,
        args::Command::Dependents(dependents) => dependents_command(&dependents, &alpm)?,
        #[cfg(feature = "completions")]
        args::Command::Completions(completions) => completions.print(),
    }

    Ok(())
}
