<script>
  // Replicates the ZenTree ASCII art stages from rpg.rs
  export let stage = 1;
  export let health = 100;

  const trees = {
    1: `
    .
   /|\\
  / | \\
 /  |  \\
    |
   ===`,
    2: `
    *
   /|\\
  / | \\
 *  |  *
  \\ | /
   ===`,
    3: `
   ***
  *****
 ** | **
*   |   *
    |
   [=]`,
    4: `
   *****
  *******
 ****|****
***  |  ***
 *   |   *
    ===`,
    5: `
  **** ****
 *** *** ***
***** | *****
 ***  |  ***
  *   |   *
    [===]`,
    6: `
 ****** ******
******* *******
***** *** *****
 ***   |   ***
  **   |   **
     [===]`,
  };

  $: ascii = trees[Math.min(stage, 6)] || trees[1];
  $: healthColor = health > 60 ? '#22c55e' : health > 30 ? '#f59e0b' : '#ef4444';
</script>

<div class="zen-tree">
  <pre class="tree-art">{ascii}</pre>
  <div class="health-bar">
    <div class="health-fill" style="width: {health}%; background: {healthColor};"></div>
  </div>
  <div class="stage-label">Stage {stage} · {health}% health</div>
</div>

<style>
  .zen-tree {
    font-family: inherit;
    text-align: center;
  }

  .tree-art {
    color: #4ade80;
    font-size: 0.75rem;
    line-height: 1.3;
    display: inline-block;
    text-align: left;
  }

  .health-bar {
    width: 120px;
    height: 4px;
    background: #1c1c1c;
    border-radius: 2px;
    margin: 0.5rem auto;
    overflow: hidden;
  }

  .health-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.5s ease;
  }

  .stage-label {
    font-size: 0.7rem;
    color: #555;
    letter-spacing: 0.05em;
  }
</style>
