// script.js — show category scores + use overall for plant growth

// Elements (matching index.html)
const uploadBox = document.querySelector('.upload-box');

const fileInput    = document.getElementById('file-upload');
const submitBtn    = document.querySelector('.submit-btn');
const scoreDisplay = document.getElementById('score-value');

const stem         = document.getElementById('stem');
const leavesGroup  = document.getElementById('leaves-group');
const flower       = document.getElementById('flower');

// NEW: fields for category scores
const scoreCleanEl   = document.getElementById('score-clean');
const scoreCorrectEl = document.getElementById('score-correct');
const scoreStructEl  = document.getElementById('score-structure');

// Optional preview elements (keep if you already added preview)
const previewWrap  = document.getElementById('file-preview');
const metaEl       = document.getElementById('file-meta');
const contentEl    = document.getElementById('file-content');

// === (Optional) file preview limits ===
const PREVIEW_MAX_BYTES = 150 * 1024;
const PREVIEW_MAX_LINES = 400;

if (fileInput) {
  fileInput.addEventListener('change', async () => {
    if (!fileInput.files || fileInput.files.length === 0) {
      if (uploadBox) uploadBox.classList.remove('expanded');
      return;
    }

    const file = fileInput.files[0];

    if (metaEl) {
      const kb = (file.size / 1024).toFixed(1);
      metaEl.textContent = `${file.name} — ${kb} KB`;
    }

    if (contentEl) {
      const blob = file.size > PREVIEW_MAX_BYTES ? file.slice(0, PREVIEW_MAX_BYTES) : file;
      const text = await blob.text();
      let lines = text.split(/\r?\n/);

      if (lines.length > PREVIEW_MAX_LINES) {
        lines = lines.slice(0, PREVIEW_MAX_LINES);
        lines.push('…(truncated for preview)…');
      }

      contentEl.textContent = lines.join('\n'); // safe (no HTML execution)

      if (uploadBox) {
        uploadBox.classList.add('expanded');
      }
    }
  });
}

// === Submit: send full file text -> backend -> update UI ===
submitBtn.addEventListener('click', async () => {
  if (!fileInput || !submitBtn) {
    alert('Page not ready: elements not found.');
    return;
  }
  if (!fileInput.files || fileInput.files.length === 0) {
    alert('Please select a file first!');
    return;
  }

  submitBtn.disabled = true;
  const originalLabel = submitBtn.textContent;
  submitBtn.textContent = 'Judging…';

  try {
    const file = fileInput.files[0];
    const text = await file.text();

    const res = await fetch('http://localhost:3000/judge', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain' },
      body: text
    });

    if (!res.ok) {
      const errText = await res.text().catch(() => '');
      throw new Error(`Backend error ${res.status}: ${errText || 'request failed'}`);
    }

    const result = await res.json();
    // result has: cleanliness, correctness, structure, overall, notes (from backend)

    // save the last judgement for detail pages
    try {
      localStorage.setItem('lastResult', JSON.stringify(result));
    } catch (e) {
      console.warn('could not write result to localStorage', e);
    }

    // === render category scores ===
    if (scoreCleanEl)   scoreCleanEl.textContent   = String(result.cleanliness ?? '—');
    if (scoreCorrectEl) scoreCorrectEl.textContent = String(result.correctness ?? '—');
    if (scoreStructEl)  scoreStructEl.textContent  = String(result.structure  ?? '—');

    // show advice messages if present
    const adviceCleanEl = document.getElementById('advice-clean');
    const adviceCorrectEl = document.getElementById('advice-correct');
    const adviceStructEl = document.getElementById('advice-structure');
    if (result.notes && Array.isArray(result.notes)) {
      result.notes.forEach(n => {
        if (!n.kind || !n.msg) return;
        if (n.kind === 'cleanliness_advice' && adviceCleanEl) {
          adviceCleanEl.textContent = n.msg;
        }
        if (n.kind === 'correctness_advice' && adviceCorrectEl) {
          adviceCorrectEl.textContent = n.msg;
        }
        if (n.kind === 'structure_advice' && adviceStructEl) {
          adviceStructEl.textContent = n.msg;
        }
      });
    }

    // Keep using the OVERALL super-score for growth animation
    updatePlantUI(result.overall ?? 0);

  } catch (err) {
    console.error(err);
    alert('Could not judge code. Is the Rust server running on port 3000?\n\n' + err);
  } finally {
    submitBtn.disabled = false;
    submitBtn.textContent = originalLabel;
  }
});

// === Plant visuals (unchanged, still based on OVERALL) ===
function updatePlantUI(score) {
  score = Math.max(0, Math.min(100, Number(score) || 0));

  // Update super-score text
  if (scoreDisplay) scoreDisplay.textContent = String(score);

  // Map overall -> plant visuals
  const heightY = 140 - score; // 0 -> 140, 100 -> 40
  const healthy = score >= 50;
  const color   = healthy ? '#2ecc71' : '#a0522d';

  if (stem) {
    stem.setAttribute('y2', String(heightY));
    stem.setAttribute('stroke', color);
  }

  if (leavesGroup) {
    leavesGroup.innerHTML = '';
    if (score > 20) {
      const leafCount = Math.floor(score / 10);
      for (let i = 0; i < leafCount; i++) {
        const side = i % 2 === 0 ? 1 : -1;
        const yPos = 140 - (i * 12);
        const cx   = 100 + 8 * side;

        const leaf = document.createElementNS('http://www.w3.org/2000/svg', 'ellipse');
        leaf.setAttribute('cx', String(cx));
        leaf.setAttribute('cy', String(yPos));
        leaf.setAttribute('rx', '10');
        leaf.setAttribute('ry', '5');
        leaf.setAttribute('fill', color);
        // Better reliability on SVG: transform attribute
        leaf.setAttribute('transform', `rotate(${25 * side} ${cx} ${yPos})`);
        leavesGroup.appendChild(leaf);
      }
    }
  }

  if (flower) {
    flower.setAttribute('r', score === 100 ? '8' : '0');
    flower.setAttribute('cy', String(heightY));
  }
}
