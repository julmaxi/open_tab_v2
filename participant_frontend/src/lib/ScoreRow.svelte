<script>
    import { createEventDispatcher } from "svelte";

    /** @type {any} */
    export let label;
    /** @type {Array<any> | null} */
    export let labelOptions = null;
    /** @type {Array<number | null>} */
    export let scores;
    /** @type number */
    export let totalScore;
    /** @type string */
    export let color;
    /** @type string */
    export let inputPrefix;
    /** @type boolean */
    export let compact = false;
    /** @type number */
    export let maxValue = 100;

	const dispatch = createEventDispatcher();

    // Don't remove. This allows tailwind to discover the classes.
    // grid-cols-4 grid-cols-3 grid-cols-2 grid-cols-1
    $: num_scores = scores.length >= 4 ? 4 : scores.length;

    $: grid_cols = `grid-cols-${num_scores}`;
</script>

<div class="flex flex-wrap">
    <div class="{color} p-1 truncate grow {compact ? "w-64" : "w-full"} md:w-64">
        {#if labelOptions === null || compact}
            {label?.name || "<Not set>"}
            <input type="hidden" name="{inputPrefix}label" value={label?.uuid || null} />
        {:else}
                <select
                    name="{inputPrefix}label"
                    class="truncate w-full appearance-none"
                    on:change={
                    (e) => {
                        dispatch(
                            'changeLabel',
                            {
                                label: e.target.value
                            }
                        )
                    }
                    }
                >
                    <option value={""} disabled selected={label?.uuid === undefined}>Select…</option>
                    {#each labelOptions as optionValue}
                        <option value={optionValue.uuid} selected={optionValue.uuid == label?.uuid} >{optionValue.name}</option>
                    {/each}
                </select>
        {/if}
    </div>

    <div class="{compact ? "" : "flex-grow"} flex mb-[-1px]">
        {#if !compact}
        <div class="grid {grid_cols} grid-flow-row flex-grow md:grid-flow-col-dense auto-cols-fr">
            {#each scores as score, scoreIdx}
                <div class="h-12 border-r border-b">
                    <input name="{inputPrefix}scores.{scoreIdx}" class="outline-none focus:ring-2 ring-inset w-full h-full text-right pr-2" type=number min=0 max={maxValue} bind:value={score} />
                </div>
            {/each}
        </div>
        {/if}

        <div class="max-w-md h-full w-16 border-b border-l ml-[-1px] font-bold">
            <input class="w-full h-full text-center" type="text" readonly value={totalScore?.toFixed(2) || "-"} tabindex="-1" />
        </div>
    </div>
</div>