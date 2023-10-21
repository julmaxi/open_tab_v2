<script>
    import Number from "$lib/Number.svelte";


    export let detailedScores;

    export let roundInfo;

    let scores = roundInfo.map(
        (round, idx) => {
            if (!detailedScores[round.tab_index]) {
                return null;
            }
            if (detailedScores[round.tab_index] !== undefined) {
                if (detailedScores[round.tab_index].team_score !== null || detailedScores[round.tab_index].score !== null || detailedScores[round.tab_index].speaker_score  !== null) {
                    return (
                        (detailedScores[round.tab_index].team_score || 0)
                        +
                        (detailedScores[round.tab_index].speaker_score || 0)
                        +
                        (detailedScores[round.tab_index].score || 0)
                    );
                }
                else {
                    return null;
                }
            }
            else {
                return null;
            }
        }
    )

    //FIXME: Team scores do not have a total (because it is a derived attribute), so we need to handle it
    //specially here. Unelegant.
</script>

<div class="flex flex-row flex-wrap">
    {#each roundInfo as round, ridx }
        <div class="text-xs rounded-sm bg-blue-50">
            {#if round.state === "Public"}
                {#if scores[ridx] !== null}
                    <Number number={
                        scores[ridx]
                    } />
                {/if}
            {:else if round.state === "Silent"}
                <span>🤫</span>
            {/if}
        </div>

        {#if ridx < roundInfo.length - 1 && scores[ridx + 1] !== null}
            <span class="text-xs">+</span>
        {/if}
    {/each}
</div>