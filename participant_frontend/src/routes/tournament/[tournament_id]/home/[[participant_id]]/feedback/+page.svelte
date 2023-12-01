<script>
    export let data;

    let numberFormat = new Intl.NumberFormat("en-US", {
        style: "decimal",
        minimumFractionDigits: 1,
        maximumFractionDigits: 1,
    });
</script>

<style>
    h2 {
        font-weight: bold;
    }

    .wrapper {
        padding: 0.5rem;
        width: 100%;
    }

    div.val {
        background-color: white;
        border-radius: 0.125rem;
        padding: 0.5rem;
        font-weight: bold;
    }

    span {
        padding-right: 0.25rem;
        position: relative;
        top: -0.1rem;
        color: #aaa;
        font-weight: normal;
    }

    span.per {
        font-size: 0.75rem;
    }

    .section {
        display: flex;
        flex-direction: column;
        align-items: center;
    }

    .list-val {
        width: 100%;
        background-color: white;
    }

    .list-val:not(:first-child) {
        border-top: 1px solid #ccc;
    }

    .val-list {
        width: 100%;
        border-radius: 0.125rem;
        overflow: hidden;
    }
</style>
<div class="wrapper">
    {#if data.summaryValues.length == 0 && data.individualValues.length == 0}
        <div class="section">
            <em>No feedback available</em>
        </div>
    {/if}

    {#each data.summaryValues as value}
        <div class="section">
            <h2>{value.question_name}</h2>

            {#if value.type == "Average"}
                <div class="val"><span>ø</span>{numberFormat.format(value.avg)}</div>
            {:else if value.type == "Percentage"}
                <div class="val"><span class="per">%</span>{numberFormat.format(value.percentage * 100)}</div>
            {/if}
        </div>
    {/each}

    {#each data.individualValues as value}
        <div class="section">
            <h2>{value.question_name}</h2>

            <div class="val-list">
            {#each value.values as val}
                <div class="list-val">{val.val}</div>
            {/each}
            </div>
        </div>
    {/each}
</div>