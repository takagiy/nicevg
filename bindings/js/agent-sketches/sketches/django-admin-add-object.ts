// Django admin add-object POST flow, based on
// https://github.com/django/django/blob/main/django/contrib/admin/options.py and
// https://docs.djangoproject.com/en/5.2/ref/contrib/admin/.
// It follows an authenticated user's add-form submission through permission checks,
// ModelForm and inline validation, persistence, admin messages and response routing.

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
    { id: "staff", class: "box", x: 20, y: 220, label: "Staff user\nPOST add form + _save" },
    { id: "adminsite", class: "box", x: 270, y: 220, label: "AdminSite.admin_view\nauthentication + CSRF" },
    { id: "addview", class: "box", x: 540, y: 220, label: "ModelAdmin.add_view\nsets change = False" },
    { id: "permission", class: "box", x: 800, y: 220, label: "has_add_permission\ncheck model permission" },
    { id: "forms", class: "box", x: 1060, y: 220, label: "get_form + get_formsets\nparent ModelForm + inlines" },
    { id: "validate", class: "box", x: 1330, y: 220, label: "form.is_valid()\nand all_valid(formsets)" },
    { id: "errors", class: "box", x: 1330, y: 440, label: "Re-render bound form\nfield + inline errors" },
    { id: "commitfalse", class: "box", x: 1060, y: 440, label: "form.save(commit=False)\nconstruct unsaved object" },
    { id: "savemodel", class: "box", x: 800, y: 440, label: "save_model(request,obj,form)\nModel.save() to database" },
    { id: "related", class: "box", x: 540, y: 440, label: "save_related(request,form,formsets)\nsave_m2m + each inline formset" },
    { id: "message", class: "box", x: 270, y: 440, label: "message_user\n“added successfully”" },
    { id: "response", class: "box", x: 20, y: 440, label: "response_add\nbutton selects destination" },
    { id: "redirect", class: "box", x: 20, y: 650, label: "HttpResponseRedirect\nobject / changelist / add page" },
  ],
  edges: [
    { from: "staff", to: "adminsite", label: "HTTP POST" },
    { from: "adminsite", to: "addview", label: "permitted request" },
    { from: "addview", to: "permission", label: "POST branch" },
    { from: "permission", to: "forms", label: "allowed: bind POST + FILES" },
    { from: "permission", to: "staff", label: "denied: permission response" },
    { from: "forms", to: "validate", label: "parent form + inline formsets" },
    { from: "validate", to: "errors", label: "invalid: show errors" },
    { from: "errors", to: "staff", label: "render change_form template" },
    { from: "validate", to: "commitfalse", label: "valid: save parent form" },
    { from: "commitfalse", to: "savemodel", label: "unsaved model instance" },
    { from: "savemodel", to: "related", label: "saved parent pk available" },
    { from: "related", to: "message", label: "m2m + inline rows persisted" },
    { from: "message", to: "response", label: "success notice queued" },
    { from: "response", to: "redirect", label: "_continue: object; _addanother: add; else changelist" },
    { from: "redirect", to: "staff", label: "302 then browser GET" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
