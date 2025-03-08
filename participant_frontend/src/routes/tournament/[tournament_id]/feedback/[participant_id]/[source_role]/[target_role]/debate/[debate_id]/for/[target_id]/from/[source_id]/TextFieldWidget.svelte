<script>
    export let maxLength = 2048;
    export let name = '';
    export let placeholder = '';
    export let rows = 4;
    export let cols = 50;
    export let initialValue = '';

    function getTextLength(text) {
        return new TextEncoder().encode(text).length;
    }

    let text = '';
    let usedChars = getTextLength(initialValue);


    function handleInput(event) {
        text = event.target.value;
        usedChars = getTextLength(text);
    }
</script>

<style>
    .indicator {
        font-size: 0.9em;
        text-align: right;
    }
    .warning {
        color: gray;
    }
    .error {
        color: red;
    }

    textarea {
        width: 100%;
        height: 10rem;
        border-radius: 0.25rem;
        padding: 0.5rem;
    }

    .container {
        width: calc(100% - 1rem);
        margin: 0.5rem;
    }
</style>

<div class="container">
    <textarea 
        name={name}
        on:input={handleInput} 
        placeholder={placeholder} 
        rows={rows} 
        cols={cols}
        value={text || initialValue} />
    {#if usedChars > maxLength}
        <div class="indicator {usedChars > maxLength ? 'error' : 'warning'}">
            {usedChars}/{maxLength} {usedChars > maxLength ? 'characters too long!' : 'characters'}
        </div>
    {/if}
</div>