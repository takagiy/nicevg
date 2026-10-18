// Gradle plugin application, from the legacy classpath/apply route to the plugins DSL.
// The diagram compares how a plugin is found and applied, then follows the shared Plugin<Project> hook
// where it adds extensions and tasks to a Project. Gradle 6.0 adds settings-level version management and included-build resolution.
// Sources: https://docs.gradle.org/current/userguide/plugins_intermediate.html
//          https://docs.gradle.org/current/userguide/plugin_management.html
//          https://docs.gradle.org/current/userguide/implementing_gradle_plugins_binary.html
//          https://gradle.org/whats-new/gradle-6/
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string };

const sketch = {
  nodes: [
    { id: "v21", class: "box", x: 20, y: 20, label: "Gradle 2.1\nplugins {} DSL introduced" },
    { id: "v60", class: "box", x: 390, y: 20, label: "Gradle 6.0\npluginManagement versions + included builds" },
    { id: "old", class: "box", x: 20, y: 150, label: "Legacy application\npre-2.1 and still supported" },
    { id: "legacy_script", class: "box", x: 20, y: 290, label: "build.gradle\nbuildscript { repositories; dependencies }" },
    { id: "repo_legacy", class: "box", x: 20, y: 430, label: "Maven / Ivy repository\nplugin implementation artifact" },
    { id: "classpath", class: "box", x: 20, y: 570, label: "Script classpath\nplugin classes made visible" },
    { id: "apply_id", class: "box", x: 20, y: 710, label: "apply plugin: 'com.example.foo'\nor plugins.apply(id)" },
    { id: "apply_class", class: "box", x: 260, y: 710, label: "apply<PluginClass>()\nclass-based application" },
    { id: "script_apply", class: "box", x: 20, y: 850, label: "apply from: 'extra.gradle'\nlegacy script plugin inclusion" },
    { id: "new", class: "box", x: 670, y: 150, label: "Declarative application\nplugins {} block" },
    { id: "settings", class: "box", x: 670, y: 290, label: "settings.gradle(.kts)\npluginManagement {}" },
    { id: "pm_repos", class: "box", x: 670, y: 430, label: "Plugin repositories\ngradlePluginPortal(), Maven, Ivy" },
    { id: "pm_versions", class: "box", x: 930, y: 430, label: "pluginManagement.plugins {}\ncentral id → version mapping (6.0)" },
    { id: "resolution", class: "box", x: 1190, y: 430, label: "resolutionStrategy.eachPlugin {}\nrequested id → useModule coordinates" },
    { id: "included", class: "box", x: 930, y: 290, label: "includeBuild('build-logic')\ncomposite plugin source (6.0)" },
    { id: "buildscript", class: "box", x: 930, y: 150, label: "plugins { id('com.example.foo')\nversion '1.0' }" },
    { id: "request", class: "box", x: 930, y: 570, label: "PluginRequest\nplugin id + optional version" },
    { id: "marker", class: "box", x: 1190, y: 570, label: "Plugin marker artifact\nid:id.gradle.plugin:version" },
    { id: "impl", class: "box", x: 1190, y: 710, label: "Implementation artifact\nmarker dependency target" },
    { id: "plugin_class", class: "box", x: 670, y: 710, label: "Plugin<Project>\nimplementation class" },
    { id: "apply_method", class: "box", x: 670, y: 850, label: "apply(Project)\nshared plugin hook" },
    { id: "project", class: "box", x: 930, y: 850, label: "Project\nplugins / extensions / tasks" },
    { id: "pluginmanager", class: "box", x: 390, y: 570, label: "PluginManager\nfinds, applies, tracks plugin IDs" },
    { id: "plugins_container", class: "box", x: 390, y: 850, label: "Project.plugins\nplugin application state" },
    { id: "extension", class: "box", x: 1190, y: 850, label: "ExtensionContainer\ncreate('foo', FooExtension)" },
    { id: "dsl", class: "box", x: 1450, y: 850, label: "FooExtension\nconfiguration DSL model" },
    { id: "tasks", class: "box", x: 1450, y: 710, label: "TaskContainer\nregister('fooTask', FooTask)" },
    { id: "task", class: "box", x: 1450, y: 570, label: "FooTask\ninputs, outputs, actions" },
    { id: "config", class: "box", x: 1450, y: 430, label: "build.gradle(.kts)\nfoo { ... } and fooTask config" },
    { id: "core", class: "box", x: 390, y: 1000, label: "Gradle build lifecycle\nsettings → projects → task execution" },
    { id: "root_decl", class: "box", x: 20, y: 1000, label: "Root plugins { ... apply false }\nresolve once, defer project application" },
    { id: "subproject", class: "box", x: 260, y: 1000, label: "Subproject plugins { id(...) }\napply to selected project" },
    { id: "buildsrc", class: "box", x: 670, y: 1000, label: "buildSrc / convention plugin\nshared build logic" },
    { id: "precompiled", class: "box", x: 930, y: 1000, label: "Precompiled script plugin\n*.gradle(.kts) compiled as plugin" },
  ],
  edges: [
    { from: "v21", to: "new" }, { from: "v60", to: "settings" }, { from: "v60", to: "included" },
    { from: "old", to: "legacy_script" }, { from: "legacy_script", to: "repo_legacy" },
    { from: "repo_legacy", to: "classpath" }, { from: "classpath", to: "apply_id" },
    { from: "classpath", to: "apply_class" }, { from: "legacy_script", to: "script_apply" },
    { from: "apply_id", to: "pluginmanager" }, { from: "apply_class", to: "pluginmanager" },
    { from: "script_apply", to: "pluginmanager" },
    { from: "new", to: "buildscript" }, { from: "settings", to: "pm_repos" },
    { from: "settings", to: "pm_versions" }, { from: "settings", to: "resolution" },
    { from: "included", to: "request" }, { from: "buildscript", to: "request" },
    { from: "pm_versions", to: "request" }, { from: "resolution", to: "request" },
    { from: "request", to: "pm_repos" }, { from: "pm_repos", to: "marker" },
    { from: "marker", to: "impl" }, { from: "included", to: "impl" },
    { from: "impl", to: "plugin_class" }, { from: "pluginmanager", to: "plugin_class" },
    { from: "pluginmanager", to: "apply_method" }, { from: "plugin_class", to: "apply_method" },
    { from: "apply_method", to: "project" }, { from: "project", to: "plugins_container" },
    { from: "apply_method", to: "extension" }, { from: "extension", to: "dsl" },
    { from: "dsl", to: "config" }, { from: "config", to: "project" },
    { from: "apply_method", to: "tasks" }, { from: "tasks", to: "task" },
    { from: "task", to: "config" }, { from: "project", to: "core" },
    { from: "root_decl", to: "request" }, { from: "root_decl", to: "subproject" },
    { from: "subproject", to: "pluginmanager" }, { from: "buildsrc", to: "plugin_class" },
    { from: "precompiled", to: "plugin_class" }, { from: "precompiled", to: "buildsrc" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
