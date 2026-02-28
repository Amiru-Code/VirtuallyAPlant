// script.js — preview chosen file + POST to backend, then animate plant

// Elements (match your index.html)
const fileInput    = document.getElementById('file-upload');
const submitBtn    = document.querySelector('.submit-btn');
const scoreDisplay = document.getElementById('score-value');

const stem         = document.getElementById('stem');
const leavesGroup  = document.getElementById('leaves-group');
const flower       = document.getElementById('flower');

// Preview panel
const previewWrap  = document.getElementById('file-preview');
const metaEl       = document.getElementById('file-meta');
const contentEl    = document.getElementById('file-content');

// Limits for preview (keeps UI snappy on big files)
const PREVIEW_MAX_BYTES = 150 * 1024;  // 150 KB
const PREVIEW_MAX_LINES = 400;

// === File preview when user picks a file ===
fileInput.addEventListener('change', async () => {
  if (!fileInput.files || fileInput.files.length === 0) {
    if (previewWrap) previewWrap.style.display = 'none';
    return;
  }
  const file = fileInput.files[0];

  // Show name + size
  if (metaEl) {
    const kb = (file.size / 1024).toFixed(1);
    metaEl.textContent = `${file.name} — ${kb} KB`;
  }

  // Read a slice if large
  const blob = file.size > PREVIEW_MAX_BYTES ? file.slice(0, PREVIEW_MAX_BYTES) : file;
  const text = await blob.text();

  // Trim lines for rendering, keep safe (no HTML execution)
  let lines = text.split(/\r?\n/);
  if (lines.length > PREVIEW_MAX_LINES) {
    lines = lines.slice(0, PREVIEW_MAX_LINES);
    lines.push('…(truncated for preview)…');
  }
  if (contentEl) contentEl.textContent = lines.join('\n');

  if (previewWrap) previewWrap.style.display = 'block';
});

// === Submit: send full file text to the Rust backend ===
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
    const text = await file.text(); // full content

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
    // result: { cleanliness, correctness, structure, overall, notes }
    updatePlantUI(result.overall ?? 0);
  } catch (err) {
    console.error(err);
    alert('Could not judge code. Is the Rust server running on port 3000?\n\n' + err);
  } finally {
    submitBtn.disabled = false;
    submitBtn.textContent = originalLabel;
  }
});

// === Plant animation (same visual logic, reliable SVG transforms) ===
function updatePlantUI(score) {
  score = Math.max(0, Math.min(100, Number(score) || 0));
  if (scoreDisplay) scoreDisplay.textContent = String(score);

  const heightY = 140 - score;               // 0->140, 100->40
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
        // Use SVG transform attribute for best compatibility
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
