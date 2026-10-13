"use strict";
const theme = document.getElementById("theme");
function applyTheme(value) {
  if (!["light", "dark", "system"].includes(value)) value = "system";
  if (value === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = value;
  theme.value = value;
}
try { applyTheme(localStorage.getItem("arc-theme") || "system"); } catch { applyTheme("system"); }
theme.addEventListener("change", () => {
  applyTheme(theme.value);
  try { localStorage.setItem("arc-theme", theme.value); } catch { /* Preference is optional. */ }
});
const test = document.getElementById("mcp-test");
if (test) test.addEventListener("click", async () => {
  const status = document.getElementById("test-status");
  const result = document.getElementById("test-result");
  test.disabled = true; result.hidden = true; status.textContent = "Test MCP en cours…";
  try {
    for (const [id, method, params] of [
      [1, "initialize", {protocolVersion: "2025-03-26", capabilities: {}, clientInfo: {name: "arcenal-poc", version: "0.1.0"}}],
      [2, "tools/list", {}],
      [3, "tools/call", {name: test.dataset.tool, arguments: {}}]
    ]) {
      const response = await fetch(test.dataset.endpoint, {method: "POST", headers: {"Content-Type": "application/json", "Accept": "application/json"}, body: JSON.stringify({jsonrpc: "2.0", id, method, params}), signal: AbortSignal.timeout(10000)});
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      if (data.error || !data.result) throw new Error(data.error?.message || "Réponse MCP invalide");
      if (method === "initialize") {
        const initialized = await fetch(test.dataset.endpoint, {method: "POST", headers: {"Content-Type": "application/json"}, body: JSON.stringify({jsonrpc: "2.0", method: "notifications/initialized"}), signal: AbortSignal.timeout(10000)});
        if (initialized.status !== 202) throw new Error("Initialisation MCP refusée");
      }
      if (method === "tools/list" && (data.result.tools.length !== 1 || data.result.tools[0].name !== test.dataset.tool)) throw new Error("Outils MCP incohérents");
      if (method === "tools/call") result.textContent = data.result.content.map(item => item.text || "").join("\n");
    }
    result.hidden = false; status.textContent = "MCP opérationnel. Connexion à Hermes : non testée.";
  } catch (error) { status.textContent = `Échec du test MCP : ${error.message}. Réessayez ou vérifiez la connexion.`; }
  finally { test.disabled = false; }
});

const chat = document.getElementById("chat-form");
if (chat) {
  const field = document.getElementById("chat-message");
  const status = document.getElementById("chat-status");
  const button = chat.querySelector('button[type="submit"]');
  const messages = document.getElementById("messages");
  function bubble(author, text, kind) {
    const item = document.createElement("article"); item.className = `bubble ${kind}`;
    const title = document.createElement("strong"); title.textContent = author;
    const content = document.createElement("p"); content.textContent = text;
    item.append(title, content); messages.append(item); item.scrollIntoView({block: "nearest"});
  }
  document.getElementById("suggestion").addEventListener("click", () => {
    field.value = "Aide-moi à préparer une offre pour un responsable automatisme en Normandie. Commence par les informations à clarifier, sans inventer de conditions.";
    field.focus();
  });
  chat.addEventListener("submit", async event => {
    event.preventDefault();
    const message = field.value.trim(); if (!message || button.disabled) return;
    button.disabled = true; field.readOnly = true;
    bubble("Vous", message, "user"); status.textContent = "Élise réfléchit dans votre Hermes…";
    try {
      const response = await fetch(chat.dataset.endpoint, {method: "POST", headers: {"Content-Type": "application/json", "X-ARCenal-Request": document.body.dataset.native === "true" ? "native" : "conversation"}, body: JSON.stringify({message}), signal: AbortSignal.timeout(130000)});
      const data = await response.json();
      if (!response.ok || data.error) throw new Error(data.error || `HTTP ${response.status}`);
      if (data.source !== "hermes" || !data.text) throw new Error("Réponse Hermes non vérifiée.");
      bubble("Élise · Hermes", data.text, "assistant"); field.value = "";
      status.textContent = "Réponse reçue de Hermes. Votre message suivant peut préciser la demande.";
    } catch (error) {
      const detail = error.name === "TimeoutError" ? "Le délai de réponse est dépassé." : error.message;
      bubble("Connexion", detail, "error"); status.textContent = `${detail} Le message est conservé pour réessayer. Aucune réponse simulée.`;
    } finally { button.disabled = false; field.readOnly = false; field.focus(); }
  });
  document.getElementById("hermes-test").addEventListener("click", async event => {
    const target = document.getElementById("hermes-status"); event.target.disabled = true; target.textContent = "Vérification en cours…";
    try {
      const response = await fetch((document.body.dataset.base || "") + "/api/hermes/status", {signal: AbortSignal.timeout(30000)});
      const data = await response.json(); if (!response.ok) throw new Error(data.error || "Hermes indisponible.");
      target.textContent = `Hermes joignable · profil ${data.profile} · ${data.model} · auteur ${data.uid}. La génération est vérifiée en envoyant un message.`;
    } catch (error) { target.textContent = `Échec : ${error.message}`; }
    finally { event.target.disabled = false; }
  });
}
