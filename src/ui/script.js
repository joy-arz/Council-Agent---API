const btn = document.getElementById('start-btn');
const browse_btn = document.getElementById('browse-btn');
const query_input = document.getElementById('query');
const workspace_input = document.getElementById('workspace');
const rounds_input = document.getElementById('rounds');
const autonomous_toggle = document.getElementById('autonomous-toggle');
const feed = document.getElementById('feed');
const status_text = document.getElementById('status-text');
const status_dot = document.getElementById('status-dot');

// Agent binary inputs
const bin_inputs = {
    strategist: document.getElementById('bin-strategist'),
    critic: document.getElementById('bin-critic'),
    optimizer: document.getElementById('bin-optimizer'),
    maintainer: document.getElementById('bin-maintainer'),
    judge: document.getElementById('bin-judge')
};

const loading_indicator = document.getElementById('loading-indicator');
const loading_text = document.getElementById('loading-text');

let current_session_id = localStorage.getItem('council_session_id');
let last_workspace = localStorage.getItem('council_workspace');

if (last_workspace) {
    workspace_input.value = last_workspace;
}

// Restore agent binaries from localStorage
Object.keys(bin_inputs).forEach(role => {
    const saved = localStorage.getItem(`council_bin_${role}`);
    if (saved) bin_inputs[role].value = saved;
});

// Test CLI functionality
document.querySelectorAll('.test-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
        const role = btn.dataset.role;
        const command = bin_inputs[role].value.trim();
        if (!command) {
            alert("Please enter a command first.");
            return;
        }

        const workspace = workspace_input.value.trim();
        btn.textContent = "Testing...";
        btn.classList.remove('success', 'error');

        try {
            const response = await fetch('/api/test_cli', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ command, workspace_dir: workspace || null })
            });
            const res = await response.json();
            if (res.status === 'success') {
                btn.textContent = "Working";
                btn.classList.add('success');
            } else {
                btn.textContent = "Failed";
                btn.classList.add('error');
                console.error(`Test failed for ${role}:`, res.message);
            }
        } catch (err) {
            btn.textContent = "Failed";
            btn.classList.add('error');
            console.error(`Fetch error for ${role}:`, err);
        }

        // Reset button after 3 seconds
        setTimeout(() => {
            btn.textContent = "Test";
            btn.classList.remove('success', 'error');
        }, 3000);
    });
});

// on load, try to restore history if session exists
if (current_session_id) {
    restore_session(current_session_id);
}

browse_btn.addEventListener('click', async () => {
    try {
        const response = await fetch('/api/browse');
        const path = await response.json();
        if (path) {
            workspace_input.value = path;
            localStorage.setItem('council_workspace', path);
        }
    } catch (err) {
        console.error("failed to browse:", err);
    }
});

btn.addEventListener('click', () => {
    const query = query_input.value.trim();
    if (!query) return;

    const workspace = workspace_input.value.trim();
    const autonomous = autonomous_toggle.checked;
    const rounds = rounds_input.value;

    if (workspace) {
        localStorage.setItem('council_workspace', workspace);
    }

    // Save binaries to localStorage
    Object.keys(bin_inputs).forEach(role => {
        localStorage.setItem(`council_bin_${role}`, bin_inputs[role].value);
    });

    btn.disabled = true;
    feed.innerHTML = '';
    
    // add user message to feed
    append_message('User', query, 0);
    query_input.value = '';
    
    update_status('Convening...', true);
    show_loading(true, 'The Council is convening...');

    start_debate(query, null, autonomous, workspace, rounds);
});

query_input.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') btn.click();
});

function show_loading(show, text = 'The Council is convening...') {
    if (show) {
        loading_indicator.classList.remove('hidden');
        loading_indicator.style.display = 'flex';
        loading_text.textContent = text;
    } else {
        loading_indicator.classList.add('hidden');
        loading_indicator.style.display = 'none';
    }
}

function update_status(text, active = false) {
    status_text.textContent = text;
    if (active) {
        status_dot.classList.add('active');
    } else {
        status_dot.classList.remove('active');
    }
}

function start_debate(query, session_id, autonomous, workspace, rounds) {
    let url = "/api/council?query=" + encodeURIComponent(query);
    if (session_id) url += "&session_id=" + session_id;
    if (autonomous) url += "&autonomous=true";
    if (workspace) url += "&workspace_dir=" + encodeURIComponent(workspace);
    if (rounds) url += "&rounds=" + rounds;

    // Add binary overrides
    Object.keys(bin_inputs).forEach(role => {
        const val = bin_inputs[role].value.trim();
        if (val) {
            url += `&${role}_binary=${encodeURIComponent(val)}`;
        }
    });

    console.log("Starting session with URL:", url);
    const event_source = new EventSource(url);

    event_source.onopen = () => {
        console.log("SSE Connection opened.");
    };

    event_source.addEventListener('session_info', (event) => {
        const data = JSON.parse(event.data);
        console.log("Session info received:", data);
        current_session_id = data.session_id;
        localStorage.setItem('council_session_id', current_session_id);
    });

    let lead_engineer_received = false;

    event_source.onmessage = (event) => {
        const data = JSON.parse(event.data);
        show_loading(false);
        append_message(data.agent, data.content, data.round, data.terminal_output);
        update_status(data.agent + " is speaking...", true);
        show_loading(true, `Waiting for Council deliberation...`);
        
        if (data.agent.toLowerCase().replace(/ /g, '.') === 'lead.engineer') {
            lead_engineer_received = true;
        }
    };

    event_source.onerror = (e) => {
        console.log("SSE Connection closed.");
        event_source.close();
        btn.disabled = false;
        show_loading(false);
        
        if (lead_engineer_received) {
            update_status('Council Adjourned', false);
            // Optionally add a small notification card
            const done_div = document.createElement('div');
            done_div.style = "text-align: center; color: var(--accent); font-size: 0.8rem; font-weight: 600; margin: 20px 0; text-transform: uppercase; letter-spacing: 0.1em;";
            done_div.textContent = "— Deliberation Complete —";
            feed.appendChild(done_div);
        } else {
            update_status('System Idle (Session Error)', false);
        }
    };
}

async function restore_session(session_id) {
    update_status('Restoring session...', true);
    try {
        const response = await fetch(`/api/history/${session_id}`);
        const history = await response.json();
        
        if (history.length > 0) {
            feed.innerHTML = '';
            history.forEach(msg => {
                append_message(msg.agent, msg.content, msg.round, msg.terminal_output);
            });
            update_status('System Idle', false);
        } else {
            localStorage.removeItem('council_session_id');
            update_status('System Idle', false);
        }
    } catch (err) {
        console.error("failed to restore session:", err);
        update_status('System Idle', false);
    }
}

function append_message(agent, content, round, terminal_output = "") {
    const card = document.createElement('div');
    const agent_lower = agent.toLowerCase().replace(/ /g, '.');
    card.className = "card " + agent_lower;

    const header = document.createElement('div');
    header.className = 'card-header';
    
    const agent_info = document.createElement('div');
    agent_info.className = 'agent-info';
    
    const icon = document.createElement('div');
    icon.className = 'agent-icon';
    icon.textContent = agent.charAt(0).toUpperCase();
    
    const name_span = document.createElement('span');
    name_span.className = 'agent-name';
    name_span.textContent = agent;
    
    agent_info.appendChild(icon);
    agent_info.appendChild(name_span);
    
    const round_tag = document.createElement('span');
    round_tag.className = 'round-tag';
    round_tag.textContent = round === 0 ? "Initial" : "Round " + round;

    header.appendChild(agent_info);
    header.appendChild(round_tag);

    const body = document.createElement('div');
    body.className = 'card-content';
    
    // check if content is json (from judge)
    try {
        if (agent_lower === 'lead.engineer') {
            const clean_content = content.replace(/```json/gi, '').replace(/```/g, '').trim();
            const json = JSON.parse(clean_content);
            const status_color = json.final_decision === 'FINISHED' ? 'var(--accent)' : (json.final_decision === 'CONTINUE' ? 'var(--primary)' : 'var(--warning)');
            body.innerHTML = `
                <div class="verdict-grid">
                    <div class="verdict-item full-width">
                        <span class="verdict-label">Summary</span>
                        <div>${json.summary}</div>
                    </div>
                    <div class="verdict-item">
                        <span class="verdict-label">Status</span>
                        <div style="color: ${status_color}; font-weight: 700; font-family: 'Fira Code';">${json.final_decision}</div>
                    </div>
                    <div class="verdict-item">
                        <span class="verdict-label">Best Answer</span>
                        <div style="font-size: 0.8rem; opacity: 0.8;">${json.best_answer}</div>
                    </div>
                    <div class="verdict-item full-width">
                        <span class="verdict-label">Reasoning</span>
                        <div style="font-style: italic; font-size: 0.85rem; color: var(--text-muted);">${json.reasoning}</div>
                    </div>
                    <div class="verdict-item full-width">
                        <span class="verdict-label">Key Disagreements</span>
                        <ul style="margin: 0; padding-left: 18px; color: var(--text-muted); font-size: 0.85rem;">
                            ${json.key_disagreements.map(d => `<li>${d}</li>`).join('')}
                        </ul>
                    </div>
                </div>
            `;
        } else if (agent_lower === 'user') {
            body.textContent = content;
        } else {
            // Parse proposals in non-autonomous mode
            const proposals = parse_proposals(content);
            if (proposals.length > 0) {
                let clean_content = content;
                proposals.forEach(p => {
                    clean_content = clean_content.replace(p.raw, "");
                });
                
                body.textContent = clean_content.trim();
                
                proposals.forEach(p => {
                    const prop_div = document.createElement('div');
                    prop_div.className = 'proposal-box';
                    prop_div.style = "margin-top: 16px; border: 1px solid var(--border); border-radius: 0; overflow: hidden;";
                    
                    const prop_header = document.createElement('div');
                    prop_header.style = "background: var(--bg); padding: 8px 12px; font-size: 0.75rem; border-bottom: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center;";
                    prop_header.innerHTML = `<span><strong style="color: var(--primary);">PROPOSAL:</strong> ${p.path}</span>`;
                    
                    const apply_btn = document.createElement('button');
                    apply_btn.textContent = "Apply Change";
                    apply_btn.className = "browse-btn"; // reuse style
                    apply_btn.style = "background: var(--accent); color: white; border: none; padding: 4px 12px;";
                    apply_btn.onclick = () => apply_proposed_change(p.path, p.content, apply_btn);
                    
                    prop_header.appendChild(apply_btn);
                    
                    const prop_body = document.createElement('pre');
                    prop_body.style = "margin: 0; padding: 12px; font-size: 0.8rem; background: #000; overflow-x: auto; font-family: 'Fira Code', monospace; color: #a5b4fc;";
                    prop_body.textContent = p.content;
                    
                    prop_div.appendChild(prop_header);
                    prop_div.appendChild(prop_body);
                    body.appendChild(prop_div);
                });
            } else {
                body.textContent = content;
            }
        }
    } catch (e) {
        body.textContent = content;
    }

    // Add terminal output if present (collapsible)
    if (terminal_output) {
        const term_details = document.createElement('details');
        term_details.style = "margin-top: 16px; font-size: 0.75rem; border-top: 1px solid var(--border); padding-top: 12px;";
        const term_summary = document.createElement('summary');
        term_summary.textContent = "Terminal Logs & Thoughts";
        term_summary.style = "cursor: pointer; color: var(--text-muted); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;";
        
        const term_pre = document.createElement('pre');
        term_pre.style = "margin-top: 8px; padding: 12px; background: #000; border-radius: 0; max-height: 400px; overflow-y: auto; overflow-x: hidden; color: #94a3b8; font-family: 'Fira Code', monospace; white-space: pre-wrap; overflow-wrap: anywhere;";
        term_pre.textContent = terminal_output;
        
        term_details.appendChild(term_summary);
        term_details.appendChild(term_pre);
        body.appendChild(term_details);
    }

    card.appendChild(header);
    card.appendChild(body);
    feed.appendChild(card);

    // auto-scroll
    feed.scrollTo({ top: feed.scrollHeight, behavior: 'smooth' });
}

function parse_proposals(text) {
    const proposals = [];
    const regex = /\[PROPOSE_CHANGE:(.*?)\]([\s\S]*?)\[\/PROPOSE_CHANGE\]/g;
    let match;
    while ((match = regex.exec(text)) !== null) {
        proposals.push({
            path: match[1].trim(),
            content: match[2].trim(),
            raw: match[0]
        });
    }
    return proposals;
}

async function apply_proposed_change(path, content, btn) {
    btn.disabled = true;
    btn.textContent = "Applying...";
    try {
        const response = await fetch('/api/apply', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ path, content })
        });
        const res = await response.json();
        if (res.status === 'success') {
            btn.textContent = "Applied";
            btn.style.background = "var(--text-muted)";
        } else {
            alert("Error: " + res.message);
            btn.disabled = false;
            btn.textContent = "Apply Change";
        }
    } catch (err) {
        alert("Failed to apply change.");
        btn.disabled = false;
        btn.textContent = "Apply Change";
    }
}
