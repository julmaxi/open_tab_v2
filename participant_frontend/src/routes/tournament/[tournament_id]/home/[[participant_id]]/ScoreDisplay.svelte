<script>
    export let info;

    let numberFormat = new Intl.NumberFormat("en-US", {
        style: "decimal",
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
    });

    console.log(info);

    let joinedInvididualScores = info?.adjudicator_scores != undefined ? info.adjudicator_scores.join(" + ") : "";
</script>

<style>
    .detail {
        font-size: 0.1rem;
        border-top: 1px #ccc solid;
        text-align: center;
    }

    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
    }
</style>


<div class="container">
    {#if info == undefined}
        -
    {:else}
        {#if info.score_status == "DidNotParticipate"}
            <em>dnc.</em>
        {:else if info.score_status == "Hidden"}
            🤫
        {:else if info.score_status == "Shown"}
            <div>
                <div>
                    <div>
                        <span>{numberFormat.format(info.total_score)}</span>
                    </div>
                </div>
                <div class="detail">
                    {joinedInvididualScores}
                </div>
            </div>
        {/if}
    {/if}
</div>