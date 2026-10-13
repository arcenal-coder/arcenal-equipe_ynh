"use strict";
(() => {
  const container = document.getElementById("recruitment-data");
  if (!container) return;
  const base = document.body.dataset.base || "";
  const native = document.body.dataset.native === "true";
  const state = {data: null, view: "candidates"};
  const status = document.getElementById("recruitment-status");
  const refresh = document.getElementById("recruitment-refresh");
  const detail = document.getElementById("recruitment-detail");
  const originalTitle = document.title;
  const normalize = value => String(value || "").normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
  function element(tag, text, className) {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = String(text);
    if (className) node.className = className;
    return node;
  }
  const date = value => {
    if (!value) return "Date non renseignée";
    const parsed = new Date(String(value).replace(" ", "T"));
    return Number.isNaN(parsed.getTime()) ? String(value) : new Intl.DateTimeFormat("fr", {dateStyle: "medium"}).format(parsed);
  };
  function empty(target, text) { target.replaceChildren(element("p", text, "empty-data")); }
  function setOptions(id, values) {
    const select = document.getElementById(id); const previous = select.value;
    const all = select.options[0].cloneNode(true); select.replaceChildren(all);
    for (const value of [...new Set(values)].filter(Boolean).sort((a,b) => a.localeCompare(b,"fr"))) {
      const option = element("option", value); option.value = value; select.append(option);
    }
    if (values.includes(previous)) select.value = previous;
  }
  function showDetail(title, fields) {
    document.getElementById("detail-title").textContent = title;
    const list = element("dl", undefined, "detail-fields");
    for (const [label,value] of fields) if (value !== undefined && value !== null && String(value).trim()) {
      list.append(element("dt", label), element("dd", Array.isArray(value) ? value.join("\n") : value));
    }
    document.getElementById("detail-content").replaceChildren(list); detail.showModal();
  }
  function candidateCard(candidate) {
    const button = element("button", undefined, "candidate-card"); button.type = "button";
    button.append(element("strong", `${candidate.first_name || ""} ${candidate.last_name || ""}`.trim()),
      element("span", candidate.position || "Candidature spontanée"), element("small", candidate.location || "Localisation non renseignée"),
      element("small", date(candidate.created_at)));
    button.addEventListener("click", () => native ? window.arcenalEdit("candidate",candidate) : showDetail(`${candidate.first_name || ""} ${candidate.last_name || ""}`, [
      ["Étape", state.data.statuses[candidate.status] || candidate.status], ["Poste", candidate.position],
      ["Lieu", candidate.location], ["E-mail", candidate.email], ["Téléphone", candidate.phone],
      ["Expérience", candidate.experience], ["Mobilité", candidate.mobility], ["Attente salariale", candidate.salary_expectation],
      ["Compétences / tags", candidate.tags], ["Motivation", candidate.motivation], ["Notes", candidate.notes],
      ["CV", candidate.cv_original], ["Lettre", candidate.cover_letter_original], ["Candidature reçue", date(candidate.created_at)],
    ]));
    return button;
  }
  function matches(candidate, query) {
    return normalize([candidate.first_name,candidate.last_name,candidate.position,candidate.location,candidate.experience,candidate.tags,candidate.notes].join(" ")).includes(normalize(query));
  }
  function jobOf(candidate) { return state.data.jobs.find(job => Number(job.id) === Number(candidate.job_id)); }
  function boardPosition(candidate) { return jobOf(candidate)?.title || candidate.position || "Candidatures spontanées"; }
  function pipeline() {
    const target = document.getElementById("candidate-pipeline"); target.replaceChildren();
    const query = document.getElementById("candidate-search").value;
    const position = document.getElementById("candidate-position").value;
    const rows = state.data.candidates.filter(c => !c.archived_at && Number(c.in_pool || 0) !== 1 && matches(c,query) && (!position || boardPosition(c) === position));
    const order = ["new","screening","interview","offer","hired","rejected"];
    for (const stage of Object.keys(state.data.statuses)) if (!order.includes(stage)) order.push(stage);
    for (const stage of order) {
      const column = element("section", undefined, "pipeline-column"); column.dataset.stage = stage;
      const items = rows.filter(c => c.status === stage);
      const heading = element("h3", state.data.statuses[stage] || stage); heading.append(element("span",items.length)); column.append(heading);
      for (const candidate of items) column.append(candidateCard(candidate));
      if (!items.length) column.append(element("p","Aucune candidature","empty-data"));
      target.append(column);
    }
  }
  function pool() {
    const target = document.getElementById("pool-results"); target.replaceChildren();
    const query = document.getElementById("pool-search").value;
    const position = document.getElementById("pool-position").value;
    const rows = state.data.candidates.filter(c => Number(c.in_pool || 0) === 1 && !c.archived_at && matches(c,query) && (!position || c.position === position));
    const summary = document.getElementById("pool-summary"); summary.replaceChildren();
    for (const [label,count] of [["Candidats",rows.length],["Métiers",new Set(rows.map(c => c.position)).size],["Disponibles",rows.filter(c => c.status !== "hired").length],["Recrutés",rows.filter(c => c.status === "hired").length]]) {
      const metric = element("div",label,"metric"); metric.append(element("strong",count)); summary.append(metric);
    }
    for (const candidate of rows) target.append(candidateCard(candidate));
    if (!rows.length) empty(target,"Aucun candidat dans le vivier pour cette recherche.");
  }
  function jobs() {
    const target = document.getElementById("job-results"); target.replaceChildren();
    const query = normalize(document.getElementById("job-search").value);
    const archived = document.getElementById("show-archived-jobs").checked;
    const labels = {draft:"Brouillon",published:"Publiée",closed:"Clôturée",archived:"Archivée"};
    const rows = state.data.jobs.filter(job => (archived || job.status !== "archived") && normalize([job.title,job.location,job.contract,job.description].join(" ")).includes(query));
    for (const job of rows) {
      const row = element("article",undefined,"data-item job-row"); const content = element("div");
      content.append(element("h3",job.title),element("p",[job.location,job.contract].filter(Boolean).join(" · ")),element("span",labels[job.status] || job.status,"badge"));
      const button = element("button","Voir l’offre"); button.type = "button";
      button.addEventListener("click", () => native ? window.arcenalEdit("job",job) : showDetail(job.title,[
        ["Statut",labels[job.status] || job.status],["Localisation",job.location],["Contrat",job.contract],
        ["Salaire minimum",job.salary_min],["Salaire maximum",job.salary_max],["Résumé",job.description],
        ["Contexte",job.context],["Objectif",job.objective],["Missions",job.missions],["Compétences",job.skills],["Affaire Dolibarr",job.dolibarr_ref],
      ]));
      row.append(content,button); target.append(row);
    }
    if (!rows.length) empty(target,"Aucune offre pour cette recherche.");
  }
  function settings() {
    const data = state.data.settings;
    if(native){const edit=document.getElementById("native-settings-edit");edit.disabled=!state.data.admin;edit.onclick=()=>window.arcenalSettings(data);}
    const target = document.getElementById("recruitment-connectors"); target.replaceChildren();
    for (const connector of data.connectors) {
      const card = element("article",undefined,"data-item"); card.append(element("h3",connector.label),element("p",connector.enabled ? "Activé" : "Désactivé"),element("p",`Mode : ${connector.mode || "non renseigné"}`)); target.append(card);
    }
    if (!data.connectors.length) empty(target,"Aucun connecteur configuré.");
    const templates = document.getElementById("recruitment-templates"); templates.replaceChildren();
    for (const [title,key] of [["Modèle de non retenue","rejectionTemplate"],["Newsletter recrutement","newsletterTemplate"]]) {
      const item = data[key] || {}; const card = element("article",undefined,"data-item");
      card.append(element("h3",title),element("strong",item.subject || "Objet non renseigné"),element("p",item.body || "Aucun modèle enregistré.","template-text")); templates.append(card);
    }
    const training = document.getElementById("recruitment-training"); training.replaceChildren(element("h3","École d’Élise"));
    const profile = data.eliseTraining?.profile || {};
    for (const [key,label] of [["policy_version","Politique"],["scoring_version","Évaluation"],["prompt_version","Instructions"]]) if (profile[key]) training.append(element("p",`${label} : ${profile[key]}`));
    if (data.matchThreshold !== undefined && data.matchThreshold !== null) training.append(element("p",`Seuil de correspondance : ${data.matchThreshold}`));
    for (const [key,label] of [["skills","Compétences"],["mobility","Mobilité"],["salary","Salaire"],["experience","Expérience"],["title","Métier"]]) {
      if (profile.weights && profile.weights[key] !== undefined) training.append(element("p",`${label} : ${profile.weights[key]}`));
    }
    training.append(element("pre",data.eliseTraining?.policyMarkdown || "Aucune politique enregistrée.","template-text"));
  }
  function activity() {
    const target = document.getElementById("recruitment-activity"); target.replaceChildren();
    const labels = {create:"Création",status:"Changement d’étape",notes:"Notes",profile:"Profil",save:"Enregistrement",settings:"Réglages",distribution:"Diffusion",interview:"Entretien",archive:"Archivage",mail:"E-mail",newsletter:"Newsletter",delete:"Suppression"};
    for (const item of state.data.activity) {
      const row = element("article",undefined,"data-item audit-row"); row.append(element("time",date(item.created_at)),element("strong",item.actor || "Auteur non renseigné"),element("p",`${labels[item.action] || item.action} · ${item.details || ""}`)); target.append(row);
    }
    if (!state.data.activity.length) empty(target,"Aucune activité enregistrée.");
  }
  function render() { if (!state.data) return; pipeline();pool();jobs();settings();activity(); }
  function select(view) {
    state.view = view;
    for (const button of document.querySelectorAll("[data-recruitment-tab]")) {
      if (button.dataset.recruitmentTab === view) button.setAttribute("aria-current","page"); else button.removeAttribute("aria-current");
    }
    for (const panel of document.querySelectorAll("[data-recruitment-view]")) panel.hidden = panel.dataset.recruitmentView !== view;
    container.hidden = view === "conversation";
    history.replaceState(null,"",`#${view}`); document.title = `${({candidates:"Candidatures",pool:"Vivier",jobs:"Offres",settings:"Réglages",activity:"Activité",conversation:"Conversation"})[view]} · Élise · ARCenal`;
  }
  async function load() {
    refresh.disabled = true; status.textContent = "Chargement des données réelles du recrutement ONYX…";
    try {
      const response = await fetch(base+"/api/recruitment",{signal:AbortSignal.timeout(60000)}); const data = await response.json();
      if (!response.ok || data.error) throw new Error(data.error || `HTTP ${response.status}`);
      if (data.source !== (native ? "arcenal" : "onyx_recruitment") || !Array.isArray(data.candidates) || !Array.isArray(data.jobs)) throw new Error("Données du recrutement non vérifiées.");
      state.data = data;
      setOptions("candidate-position",data.candidates.filter(c => !c.archived_at && Number(c.in_pool || 0) !== 1).map(boardPosition));
      setOptions("pool-position",data.candidates.filter(c => !c.archived_at && Number(c.in_pool || 0) === 1).map(c => c.position));
      render(); status.textContent = native ? `Données natives actualisées · ${data.candidates.length} dossiers · ${data.jobs.length} offres.` : `Données ONYX actualisées · ${data.candidates.length} dossiers · ${data.jobs.length} offres. Consultation ; les modifications restent dans votre application existante.`;
      container.dataset.loaded = "true";
    } catch (error) { status.textContent = `${error.message} Aucune donnée simulée.`; container.dataset.loaded = "error"; }
    finally { refresh.disabled = false; }
  }
  for (const button of document.querySelectorAll("[data-recruitment-tab]")) button.addEventListener("click",()=>select(button.dataset.recruitmentTab));
  for (const id of ["candidate-search","candidate-position","pool-search","pool-position","job-search","show-archived-jobs"]) document.getElementById(id).addEventListener("input",render);
  refresh.addEventListener("click",load); document.getElementById("detail-close").addEventListener("click",()=>detail.close());
  document.addEventListener("native-saved",load);
  const view = location.hash.slice(1); select(["candidates","pool","jobs","settings","activity","conversation"].includes(view) ? view : "candidates");
  document.title = document.title || originalTitle; load();
})();
