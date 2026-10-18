// Flask-Login: https://flask-login.readthedocs.io/en/latest/
// https://github.com/maxcountryman/flask-login/blob/main/src/flask_login/login_manager.py
// Shows how the extension attaches to a Flask app, exposes current_user to templates, and calls
// an application-provided user_loader when current_user is first resolved in a request.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string };

export default {
  nodes: [
    { id: "flask-app", class: "box", x: 40, y: 130, label: "Flask app" },
    { id: "login-manager", class: "box", x: 270, y: 80, label: "Flask-Login\nLoginManager" },
    { id: "context-processor", class: "box", x: 520, y: 30, label: "_user_context_processor\n(app.context_processor)" },
    { id: "current-user", class: "box", x: 770, y: 30, label: "current_user\nLocalProxy → _get_user()" },
    { id: "load-user", class: "box", x: 770, y: 190, label: "LoginManager._load_user()" },
    { id: "session", class: "box", x: 520, y: 280, label: "Flask session\n_user_id" },
    { id: "user-loader", class: "box", x: 270, y: 300, label: "application @user_loader\n(str user_id) → User | None" },
    { id: "user-store", class: "box", x: 40, y: 310, label: "application user lookup\n(e.g. User.get(id))" },
  ],
  edges: [
    { from: "flask-app", to: "login-manager", label: "init_app(app): app.login_manager = self" },
    { from: "login-manager", to: "flask-app", label: "registers context processor" },
    { from: "flask-app", to: "context-processor", label: "render template context" },
    { from: "context-processor", to: "current-user", label: "current_user template variable" },
    { from: "current-user", to: "load-user", label: "request context: _get_user() loads lazily" },
    { from: "session", to: "load-user", label: "session.get('_user_id')" },
    { from: "load-user", to: "user-loader", label: "_user_callback(user_id)" },
    { from: "user-loader", to: "user-store", label: "application resolves ID" },
    { from: "user-store", to: "user-loader", label: "User or None" },
    { from: "user-loader", to: "load-user", label: "callback result" },
    { from: "load-user", to: "current-user", label: "stores user at g._login_user" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
