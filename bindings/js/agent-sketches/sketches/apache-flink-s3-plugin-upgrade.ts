// Apache Flink S3 filesystem plugin and framework upgrade, based on the filesystem plugin,
// S3 filesystem, savepoint, and upgrade documentation plus Flink core plugin source.
// Follows plugin discovery into FileSystem, then an in-place upgrade that saves and restores
// streaming job state through the S3 filesystem plugin.
// https://nightlies.apache.org/flink/flink-docs-stable/docs/deployment/filesystems/plugins/
// https://nightlies.apache.org/flink/flink-docs-stable/docs/deployment/filesystems/s3/
// https://nightlies.apache.org/flink/flink-docs-master/docs/ops/upgrading/
// https://github.com/apache/flink/blob/master/flink-core/src/main/java/org/apache/flink/core/plugin/DirectoryBasedPluginFinder.java
// https://github.com/apache/flink/blob/master/flink-core/src/main/java/org/apache/flink/core/plugin/DefaultPluginManager.java
// https://github.com/apache/flink/blob/master/flink-core/src/main/java/org/apache/flink/core/fs/FileSystem.java

import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Node = SketchNode & {
  class: "component" | "artifact" | "command" | "state";
  label: string;
  width: number;
  height: number;
  fill?: string;
};

const component = ({ label, width, height, fill = "#eff6ff" }: Node) => {
  const title = text(label, 13);
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="10" fill="${fill}" stroke="#334155"/>${title.at({ cx: width / 2, cy: height / 2 })}`;
};

const artifact = ({ label, width, height, fill = "#fef3c7" }: Node) => {
  const title = text(label, 12);
  return svg`<path data-bounds="true" d="M 0 0 H ${width - 18} L ${width} 18 V ${height} H 0 Z" fill="${fill}" stroke="#92400e"/><path d="M ${width - 18} 0 V 18 H ${width}" fill="none" stroke="#92400e"/>${title.at({ cx: width / 2, cy: height / 2 })}`;
};

const command = ({ label, width, height }: Node) => {
  const title = text(label, 12);
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="${height / 2}" fill="#dcfce7" stroke="#166534"/>${title.at({ cx: width / 2, cy: height / 2 })}`;
};

const state = ({ label, width, height }: Node) => {
  const title = text(label, 12);
  return svg`<path data-bounds="true" d="M 0 12 Q ${width / 2} 0 ${width} 12 V ${height - 12} Q ${width / 2} ${height} 0 ${height - 12} Z" fill="#f3e8ff" stroke="#6b21a8"/><ellipse cx="${width / 2}" cy="12" rx="${width / 2}" ry="12" fill="#faf5ff" stroke="#6b21a8"/>${title.at({ cx: width / 2, cy: height / 2 })}`;
};

const nodes: Node[] = [
  { id: "pluginjar", class: "artifact", x: 35, y: 55, width: 205, height: 70, label: "opt/flink-s3-fs-hadoop.jar" },
  { id: "plugindir", class: "artifact", x: 35, y: 165, width: 205, height: 75, label: "plugins/s3-fs-hadoop/\nplugin JAR directory" },
  { id: "finder", class: "component", x: 300, y: 165, width: 230, height: 75, label: "DirectoryBasedPluginFinder\nsubfolder → plugin descriptor" },
  { id: "descriptor", class: "component", x: 590, y: 165, width: 200, height: 75, label: "PluginDescriptor\nplugin id + JAR URLs" },
  { id: "manager", class: "component", x: 850, y: 165, width: 210, height: 75, label: "DefaultPluginManager\nload(FileSystemFactory.class)" },
  { id: "loader", class: "component", x: 1115, y: 165, width: 205, height: 75, label: "PluginLoader\nServiceLoader per plugin" },
  { id: "classloader", class: "component", x: 1115, y: 290, width: 205, height: 70, label: "PluginClassLoader\nisolated plugin dependencies" },
  { id: "spi", class: "artifact", x: 850, y: 290, width: 210, height: 75, label: "META-INF/services/\nFileSystemFactory" },
  { id: "adapter", class: "component", x: 590, y: 290, width: 200, height: 75, label: "PluginFileSystemFactory\nkeeps calls in plugin loader" },
  { id: "initialize", class: "component", x: 300, y: 290, width: 230, height: 75, label: "FileSystem.initialize(config,\npluginManager)" },
  { id: "s3fs", class: "component", x: 300, y: 415, width: 230, height: 75, label: "S3 FileSystemFactory\nregisters s3:// scheme" },
  { id: "lookup", class: "component", x: 590, y: 415, width: 200, height: 75, label: "FileSystem.get(s3 URI)\nselects registered factory" },
  { id: "savepoint", class: "state", x: 850, y: 415, width: 210, height: 75, label: "s3://bucket/savepoints/\nconsistent operator state" },
  { id: "stop", class: "command", x: 35, y: 570, width: 205, height: 72, label: "bin/flink stop\n--savepointPath s3://… JOB_ID" },
  { id: "oldjob", class: "component", x: 300, y: 570, width: 230, height: 72, label: "Running streaming job\nold Flink version" },
  { id: "replace", class: "command", x: 590, y: 570, width: 200, height: 72, label: "Stop cluster; replace\nFlink installation + plugin" },
  { id: "newcluster", class: "component", x: 850, y: 570, width: 210, height: 72, label: "Restart cluster\nnew Flink version" },
  { id: "resume", class: "command", x: 1115, y: 570, width: 205, height: 72, label: "bin/flink run -s\ns3://…/savepoint …jar" },
  { id: "newjob", class: "component", x: 1115, y: 695, width: 205, height: 72, label: "Resumed job\nrestored operator state" },
];

const edges = [
  { from: "pluginjar", to: "plugindir", label: "copy from opt before startup" },
  { from: "plugindir", to: "finder" },
  { from: "finder", to: "descriptor" },
  { from: "descriptor", to: "manager" },
  { from: "manager", to: "loader" },
  { from: "loader", to: "classloader" },
  { from: "spi", to: "loader", label: "service provider name" },
  { from: "classloader", to: "adapter", label: "factory instance + classloader context" },
  { from: "adapter", to: "initialize", label: "loaded plugin factories" },
  { from: "initialize", to: "s3fs", label: "configure; register scheme s3" },
  { from: "s3fs", to: "lookup" },
  { from: "lookup", to: "savepoint", label: "S3 FileSystem writes savepoint files" },
  { from: "stop", to: "oldjob", label: "trigger final consistent savepoint; then stop" },
  { from: "oldjob", to: "savepoint", label: "operator state snapshot" },
  { from: "savepoint", to: "replace", label: "path must stay accessible at same absolute URI" },
  { from: "replace", to: "newcluster", label: "install new version with plugin under plugins/" },
  { from: "newcluster", to: "resume" },
  { from: "resume", to: "savepoint", label: "read state using S3 plugin" },
  { from: "resume", to: "newjob", label: "initialize operators from savepoint" },
];

export default { nodes, edges, style: { component, artifact, command, state }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
