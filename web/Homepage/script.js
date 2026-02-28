// script.js — show category scores + use overall for plant growth

// Elements (matching index.html)
const uploadBox = document.querySelector('.upload-box');

const fileInput    = document.getElementById('file-upload');
const submitBtn    = document.querySelector('.submit-btn');
const scoreDisplay = document.getElementById('score-value');

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

  // Update score text
  if (scoreDisplay) scoreDisplay.textContent = String(score);

  // Convert 0–100 into 1–10 bucket
  // 100–90 = 10, 89–80 = 9, ..., 9–0 = 1
  const stage = Math.max(1, Math.ceil(score / 10));
  let num = Math.abs(10 - stage);

  console.log(num)
  if (num === 0) {
    num = 1;
  }
  // Build filename (plant1.png → plant10.png)
  const imageName = `img/plant${num}.png`;

  // Swap the image
  const img = document.getElementById("plant-image");
  if (img) img.src = imageName;
}
