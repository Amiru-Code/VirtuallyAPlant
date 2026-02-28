const stem = document.getElementById('caule');
const scoreDisplay = document.getElementById('nota-display');
const leavesGroup = document.getElementById('folhas-grupo');
const flower = document.getElementById('flor');
const fileInput = document.getElementById('file-upload');
const submitBtn = document.querySelector('.submit-button');

// Event listener for the Submit button
submitBtn.addEventListener('click', () => {
    if (fileInput.files.length > 0) {
        // Simulation: generate a score from 0 to 100
        // In the hackathon, this will come from your backend API
        const randomScore = Math.floor(Math.random() * 101);
        updatePlantUI(100);
    } else {
        alert("Please select a file first!");
    }
});

function updatePlantUI(score) {
    // Update the text percentage (XX%)
    scoreDisplay.innerText = score;
    
    // 1. Calculate height and color based on score (0-100)
    // Map score to Y coordinate (140 is pot level, 40 is top)
    const heightY = 140 - (score * 1); 
    const isHealthy = score > 50;
    const plantColor = isHealthy ? '#2ecc71' : '#a0522d'; 
    
    // 2. Animate the Stem
    stem.setAttribute('y2', heightY);
    stem.style.stroke = plantColor;

    // 3. Generate Leaves dynamically
    leavesGroup.innerHTML = ''; 
    if (score > 20) {
        // Create 1 leaf for every 10 points
        const leafCount = Math.floor(score / 10);
        for (let i = 0; i < leafCount; i++) {
            const yPos = 140 - (i * 12);
            const side = i % 2 === 0 ? 1 : -1; 
            
            const newLeaf = document.createElementNS("http://www.w3.org/2000/svg", "ellipse");
            newLeaf.setAttribute("cx", 100 + (8 * side));
            newLeaf.setAttribute("cy", yPos);
            newLeaf.setAttribute("rx", 10);
            newLeaf.setAttribute("ry", 5);
            newLeaf.setAttribute("class", "folha");
            newLeaf.style.fill = plantColor;
            newLeaf.style.transform = `rotate(${25 * side}deg)`;
            newLeaf.style.transformOrigin = `${100 + (8 * side)}px ${yPos}px`;
            
            leavesGroup.appendChild(newLeaf);
        }
    }

    // 4. Show flower if score is 100
    flower.setAttribute("r", score === 100 ? 12 : 0);
    flower.setAttribute("cy", heightY);
}