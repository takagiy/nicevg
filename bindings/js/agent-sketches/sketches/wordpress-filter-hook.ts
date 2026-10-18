// WordPress plugin filter hook, following the Plugin Handbook and the hook implementation.
// It traces a plugin callback from registration in `add_filter()` into the `$wp_filter` registry,
// then through `apply_filters()` and `WP_Hook` as each callback transforms and returns a value.
// Sources: https://developer.wordpress.org/plugins/hooks/
//          https://developer.wordpress.org/plugins/hooks/filters/
//          https://github.com/WordPress/wordpress-develop/blob/trunk/src/wp-includes/plugin.php
//          https://github.com/WordPress/wordpress-develop/blob/trunk/src/wp-includes/class-wp-hook.php
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "plugin", class: "box", x: 30, y: 40, label: "Plugin bootstrap\nadd_filter( 'the_content', 'append_note', 10, 1 )" },
    { id: "registration", class: "box", x: 360, y: 40, label: "add_filter()\nadd_action() is an alias\nregisters callback + priority + accepted_args" },
    { id: "registry", class: "box", x: 700, y: 40, label: "$wp_filter['the_content']\nWP_Hook::$callbacks\nkeyed by priority, then unique ID" },
    { id: "core", class: "box", x: 700, y: 230, label: "Core content path\napply_filters( 'the_content', $content )" },
    { id: "dispatcher", class: "box", x: 360, y: 230, label: "WP_Hook::apply_filters()\niterate callbacks in priority order\npass current value + accepted args" },
    { id: "callback", class: "box", x: 30, y: 230, label: "Plugin callback\nappend_note( $content )\nreturns modified content" },
    { id: "continued", class: "box", x: 700, y: 420, label: "Core continues\nfiltered content used for output" },
  ],
  edges: [
    { from: "plugin", to: "registration", label: "hook name + callable + priority + arg count" },
    { from: "registration", to: "registry", label: "WP_Hook::add_filter() stores callback" },
    { from: "core", to: "dispatcher", label: "$content + extra args" },
    { from: "registry", to: "dispatcher", label: "callback entries for 'the_content'" },
    { from: "dispatcher", to: "callback", label: "current value + accepted arguments" },
    { from: "callback", to: "dispatcher", label: "replacement value" },
    { from: "dispatcher", to: "continued", label: "final filtered value" },
    { from: "continued", to: "core", label: "returned to caller; normal flow resumes" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
