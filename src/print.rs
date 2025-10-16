// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL-1.2 OR GPL-3.0
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

//! Utilities for printing packages.

use std::fmt::Display;
use std::io::prelude::*;

use anstyle::{AnsiColor, Reset, Style};
use pacgraph::graph::{DependencyEdge, PackageNode};
use petgraph::{
    dot::{Config, Dot, RankDir},
    visit::{
        Data, EdgeRef, GraphProp, IntoEdgeReferences, IntoNodeReferences, NodeIndexable, NodeRef,
    },
};

/// Print a package node in one line.
#[derive(Debug, Clone)]
pub struct DisplayPackageAnsi<P> {
    package: P,
    with_version: bool,
}

impl<P> DisplayPackageAnsi<P> {
    pub fn new(package: P) -> Self {
        Self {
            package,
            with_version: false,
        }
    }

    pub fn with_version(mut self, with_version: bool) -> Self {
        self.with_version = with_version;
        self
    }
}

impl Display for DisplayPackageAnsi<&alpm::Package> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.with_version {
            let bold = Style::new().bold();
            let green = bold.fg_color(Some(AnsiColor::Green.into()));
            write!(
                f,
                "{bold}{} {green}{}{Reset}",
                self.package.name(),
                self.package.version()
            )
        } else {
            write!(f, "{}", self.package.name())
        }
    }
}

/// Print a package graph as dot.
pub fn print_package_graph<'a, G, W: Write>(
    write: &mut W,
    graph: G,
    with_version: bool,
) -> std::io::Result<()>
where
    G: GraphProp
        + Data<NodeWeight = PackageNode<'a>, EdgeWeight = DependencyEdge>
        + IntoEdgeReferences
        + IntoNodeReferences
        + NodeIndexable,
{
    let get_node_attributes = |_graph, node: G::NodeRef| {
        let package = node.weight();
        if with_version {
            format!(
                "label = <<FONT FACE=\"sans-serif\"><B>{name} <FONT COLOR=\"green\">{version}</FONT></B></FONT>>",
                name = package.name(),
                version = package.version()
            )
        } else {
            format!(
                "label = <<FONT FACE=\"sans-serif\">{}</FONT>>",
                package.name()
            )
        }
    };
    let dot = Dot::with_attr_getters(
        graph,
        &[
            Config::EdgeNoLabel,
            Config::NodeNoLabel,
            Config::RankDir(RankDir::TB),
        ],
        &|_graph, edge| match *edge.weight() {
            DependencyEdge::Required => "style = solid".to_string(),
            DependencyEdge::Optional => "style = dashed".to_string(),
        },
        &get_node_attributes,
    );
    writeln!(write, "{dot}")
}
