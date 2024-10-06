<script>
    import { createEventDispatcher } from 'svelte';
  
    export let searchFunction;
    
    let searchTerm = '';
    let candidates = [];
    let selectedCandidate = null;
    let isOpen = false;
  
    const dispatch = createEventDispatcher();
  
    async function handleInput() {
      if (searchTerm.length > 0) {
        candidates = await searchFunction(searchTerm);
        isOpen = true;
      } else {
        candidates = [];
        isOpen = false;
      }
    }
  
    function handleSelect(candidate) {
      selectedCandidate = candidate;
      searchTerm = candidate;
      isOpen = false;
      dispatch('select', { selected: candidate });
    }
  
    function handleKeydown(event) {
      if (event.key === 'ArrowDown' && isOpen) {
        event.preventDefault();
        const currentIndex = candidates.indexOf(selectedCandidate);
        const nextIndex = (currentIndex + 1) % candidates.length;
        selectedCandidate = candidates[nextIndex];
      } else if (event.key === 'ArrowUp' && isOpen) {
        event.preventDefault();
        const currentIndex = candidates.indexOf(selectedCandidate);
        const prevIndex = (currentIndex - 1 + candidates.length) % candidates.length;
        selectedCandidate = candidates[prevIndex];
      } else if (event.key === 'Enter' && selectedCandidate) {
        handleSelect(selectedCandidate);
      }
    }
  </script>
  
  <div class="search-combobox">
    <input
      type="text"
      bind:value={searchTerm}
      on:input={handleInput}
      on:keydown={handleKeydown}
      placeholder="Search..."
    />
    {#if isOpen}
      <ul class="candidates-list">
        {#each candidates as candidate}
          <li
            class:selected={candidate === selectedCandidate}
            on:click={() => handleSelect(candidate)}
          >
            {candidate}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  
  <style>
    .search-combobox {
      position: relative;
      width: 300px;
    }
  
    input {
      width: 100%;
      padding: 8px;
      border: 1px solid #ccc;
      border-radius: 4px;
    }
  
    .candidates-list {
      position: absolute;
      width: 100%;
      max-height: 200px;
      overflow-y: auto;
      list-style-type: none;
      padding: 0;
      margin: 0;
      border: 1px solid #ccc;
      border-top: none;
      border-radius: 0 0 4px 4px;
      background-color: white;
    }
  
    .candidates-list li {
      padding: 8px;
      cursor: pointer;
    }
  
    .candidates-list li:hover,
    .candidates-list li.selected {
      background-color: #f0f0f0;
    }
  </style>