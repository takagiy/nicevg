/**
 * Unreal Engine Gameplay Ability System:
 * https://dev.epicgames.com/documentation/unreal-engine/understanding-the-unreal-engine-gameplay-ability-system
 * https://dev.epicgames.com/documentation/unreal-engine/gameplay-attributes-and-attribute-sets-for-the-gameplay-ability-system-in-unreal-engine
 * This sketch follows an Actor's ability-system state: granted ability specs,
 * attached attribute sets, and active Gameplay Effects that modify attributes.
 */
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
    { id: "actor", class: "box", x: 40, y: 140, label: "Owning Actor\nowner of ASC" },
    { id: "asc", class: "box", x: 260, y: 120, label: "UAbilitySystemComponent\nability specs · active effects\ntags · attributes" },
    { id: "spec", class: "box", x: 520, y: 35, label: "FGameplayAbilitySpec\nability class · level\ninput / activation state" },
    { id: "ability", class: "box", x: 790, y: 35, label: "UGameplayAbility\nactivation rules · tags\nexecution behavior" },
    { id: "effect", class: "box", x: 520, y: 225, label: "UGameplayEffect\nasset definition\ninstant or duration" },
    { id: "active", class: "box", x: 790, y: 225, label: "Active Gameplay Effect\nASC handle · source / target\nremaining duration / stacks" },
    { id: "set", class: "box", x: 260, y: 355, label: "UAttributeSet\nregistered with ASC" },
    { id: "attribute", class: "box", x: 560, y: 370, label: "FGameplayAttributeData\nBaseValue · CurrentValue\ne.g. Health" },
  ] satisfies Node[],
  edges: [
    { from: "actor", to: "asc", label: "owns / exposes" },
    { from: "asc", to: "spec", label: "grants and tracks spec handle" },
    { from: "spec", to: "ability", label: "references ability class" },
    { from: "ability", to: "effect", label: "creates/applies to target" },
    { from: "effect", to: "active", label: "definition instantiated on target ASC" },
    { from: "asc", to: "active", label: "stores while ongoing" },
    { from: "asc", to: "set", label: "registers one or more sets" },
    { from: "set", to: "attribute", label: "contains reflected attribute properties" },
    { from: "active", to: "attribute", label: "modifies CurrentValue; expires and is undone" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
