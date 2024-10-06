<script>
    //https://svelte.dev/repl/144f22d18c6943abb1fdd00f13e23fde?version=3.49.0
    export let disabled = undefined;
    export let error = undefined;
    export let expand = true;
    export let label = "";
    export let loading = false;
    export let name;
    export let id = "";
    export let options = [];
    export let placeholder = undefined;
    export let readonly = undefined;
    export let required = undefined;
    export let value = "";
  
    export let filter = (text) => {
      const sanitized = text.trim().toLowerCase();
          
          return options.reduce((a, o) => {
              let match;
              
              if (o.options) {
                  const options = o.options.filter((o) => o.text.toLowerCase().includes(sanitized));
                  
                  if (options.length) {
                      match = { ...o, options }
                  }
              } else if (o.text.toLowerCase().includes(sanitized)) {
                  match = o;
              }
              
              match && a.push(match);
              
              return a;
          }, [])
    };
  
    let listElement;
    let inputElement;
    let list = [];
    let isListOpen = false;
      let selectedOption;
  
    async function onInputKeyup(event) {
      switch (event.key) {
        case "Escape":
        case "ArrowUp":
        case "ArrowLeft":
        case "ArrowRight":
        case "Enter":
        case "Tab":
        case "Shift":
          break;
        case "ArrowDown":
          await showList(event.target.value);
          listElement.querySelector(`[role="option"]:not([aria-disabled="true"])`)?.focus();
  
          event.preventDefault();
          event.stopPropagation();
          break;
  
        default:
          await showList(event.target.value);
      }
    }
  
    function onInputKeydown(event) {
      let flag = false;
  
      switch (event.key) {
        case "Escape":
          hideList();
          flag = true;
          break;
  
        case "Tab":
          hideList();
          break;
      }
  
      if (flag) {
        event.preventDefault();
        event.stopPropagation();
      }
    }
  
    async function onInputClick(event) {
      await showList(event.target.value);
      // Scroll selected option into view.
      listElement.querySelector(`[role="option"][data-value="${value}"]`)?.scrollIntoView();
    }
  
    function onOptionClick(event) {
          if (!event.target.matches(`[role="option"]:not([aria-disabled="true"])`)) return
          
          selectOption(event.target);
          hideList();
    }
  
    function onListKeyDown(event) {
      let flag = false;
  
      switch (event.key) {
        case "ArrowUp":
          let prevOptionElement = event.target.previousElementSibling;
  
          while (prevOptionElement) {
            if (prevOptionElement.matches(`[role="option"]:not([aria-disabled="true"])`)) break;
            prevOptionElement = prevOptionElement.previousElementSibling;
          }
  
          prevOptionElement?.focus();
          flag = true;
          break;
  
        case "ArrowDown":
          let nextOptionElement = event.target.nextElementSibling;
  
          while (nextOptionElement) {
            if (nextOptionElement.matches(`[role="option"]:not([aria-disabled="true"])`)) break;
            nextOptionElement = nextOptionElement.nextElementSibling;
          }
  
          nextOptionElement?.focus();
          flag = true;
          break;
  
        case "Enter":
          selectOption(event.target);
          hideList();
          flag = true;
          break;
  
        case "Escape":
          hideList();
          flag = true;
          break;
  
        case "Tab":
          hideList();
          break;
  
        default:
          inputElement.focus();
      }
  
      if (flag) {
        event.preventDefault();
        event.stopPropagation();
      }
    }
  
    async function showList(inputValue) {
      const isExactMatch = options.some((o) =>
        o.options ? o.options.some((o) => o.text === inputValue) : o.text === inputValue
      );
  
      list = inputValue === "" || isExactMatch ? options : await filter(inputValue);
          isListOpen = true;
    }
  
    function hideList() {
      if (!isListOpen) return;
  
      if (selectedOption) {
        inputElement.value = selectedOption.text;
      }
  
      isListOpen = false;
      inputElement.focus();
    }
  
    function selectOption(optionElement) {
          value = optionElement.dataset.value;
          
      selectedOption = {
        text: optionElement.dataset.text,
        value: optionElement.dataset.value
      };
      }
  </script>
  
  <div class="combobox">
    <label class="combobox__label label" for={id}>
      {label}
      {#if error}
        <span class="form-validation-error">
          {error}
        </span>
      {/if}
    </label>
  
    <div class="input-container">
      <slot name="icon-start" />
  
      <input
        bind:this={inputElement}
        on:focus
        on:blur={hideList}
        on:input
        on:keyup={onInputKeyup}
        on:keydown={onInputKeydown}
        on:mousedown={onInputClick}
        class="combobox__input"
        {id}
        {name}
        type="text"
        {disabled}
        autocapitalize="none"
        autocomplete="off"
        {readonly}
        {placeholder}
        spellcheck="false"
              role="combobox"
              aria-autocomplete="list"
              aria-expanded={isListOpen}
        aria-required={required ? "true" : undefined}
      />
  
      <ul
        class="combobox__list"
        role="listbox"
              aria-label={label}
        hidden={!isListOpen}
        on:click={onOptionClick}
        on:keydown={onListKeyDown}
        bind:this={listElement}
      >
        {#each list as option (option)}
          {#if option.options}
            <li class="list__option-heading">
                          <slot name="group" group={option}>
                              {option.text}
                          </slot>
            </li>
            {#each option.options as option (option)}
              <li
                class="list__option"
                              class:--disabled={option.disabled}
                role="option"
                tabindex={option.disabled ? undefined : "-1"}
                data-text={option.text}
                data-value={option.value}
                aria-selected={value === option.value}
                              aria-disabled={option.disabled}
              >
                <slot name="option" {option}>
                  {option.text}
                </slot>
                              {#if option.value === value}
                  <svg viewBox="0 0 24 24" class="icon">
                                    <polyline points="20 6 9 17 4 12"></polyline>
                                  </svg>
                {/if}
              </li>
            {/each}
          {:else}
            <li
              class="list__option"
                          class:--disabled={option.disabled}
              role="option"
              tabindex={option.disabled === true ? undefined : "-1"}
              data-text={option.text}
              data-value={option.value}
              aria-selected={value === option.value}
                          aria-disabled={option.disabled}
            >
                  <slot name="option" {option}>
                  {option.text}
                </slot>
                              {#if option.value === value}
                  <svg viewBox="0 0 24 24" class="icon">
                                    <polyline points="20 6 9 17 4 12"></polyline>
                                  </svg>
                {/if}
            </li>
          {/if}
              {:else}
                  <li class="list__no-results">
                      No results available
                  </li>
        {/each}
      </ul>
  
      <div class="visually-hidden" role="status" aria-live="polite">
        {list.length} results available.
      </div>
    </div>
  </div>

<style>
    .combobox {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        position: relative;
    }
    
    .combobox__label {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    
    .combobox__input {
        padding: 0.5rem;
        border-radius: 0.25rem;
        border: 1px solid #aaa;
        background-color: white;
        width: 100%;
    }
    
    .combobox__list {
        list-style-type: none;
        padding: 0;
        margin: 0;
        border-radius: 0.25rem;
        background-color: white;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
        max-height: 10rem;
        overflow-y: auto;
        position: absolute;
        width: 100%;
    }
    
    .list__option {
        padding: 0.5rem;
        cursor: pointer;
    }
    
    .list__option-heading {
        padding: 0.5rem;
        font-weight: bold;
    }
    
    .list__option.--disabled {
        color: #aaa;
        cursor: not-allowed;
    }
    
    .list__option.--disabled:hover {
        background-color: transparent;
    }
    
    .list__option:hover {
        background-color: #f0f0f0;
    }
    
    .list__option.--selected {
        background-color: #f0f0f0;
    }
    
    .list__no-results {
        padding: 0.5rem;
    }
    
    .visually-hidden {
        position: absolute;
        width: 1px;
        height: 1px;
        margin: -1px;
        padding: 0;
        overflow: hidden;
        clip: rect(0, 0, 0, 0);
        border: 0;
    }
    
    .icon {
        width: 1rem;
        height: 1rem;
        fill: none;
        stroke: currentColor;
        stroke-width: 2;
    }
</style>