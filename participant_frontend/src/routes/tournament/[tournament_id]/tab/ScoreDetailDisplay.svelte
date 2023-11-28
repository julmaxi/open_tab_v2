<script>
    import Number from "$lib/Number.svelte";

    export let detailedScores;

    let scores = detailedScores.map((score, idx) => {
        if (score != undefined) {
            if (
                score.team_score !== null ||
                score.score !== null ||
                score.speaker_score !== null
            ) {
                return (
                    (score.team_score || 0) +
                    (score.speaker_score || 0) +
                    (score.score || 0)
                );
            } else {
                return null;
            }
        } else {
            return null;
        }
    });

    let roleClasses = detailedScores.map((score, idx) => {
        if (score != undefined) {
            switch (score.role || score.team_role) {
                case "Government":
                    return "gov";
                case "Opposition":
                    return "opp"
                case "NonAligned":
                    return "non-aligned";
            }
        } else {
            return "";
        }
    });
    //FIXME: Team scores do not have a total (because it is a derived attribute), so we need to handle it
    //specially here. Unelegant.
</script>

<div class="container">
    {#each detailedScores as score, ridx}
        {#if scores[ridx] !== null}
            <div class="score-box">
                <div class={roleClasses[ridx]}>
                    <Number number={scores[ridx]} />
                </div>
            </div>
        {/if}
    {/each}
</div>

<style>
    .container {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
    }

    .score-box:not(:last-child)::after {
        content: "+";
        display: inline;
        padding-right: 0.1rem;
    }

    .score-box {
        font-size: 0.75rem;
        line-height: 1rem;
    }

    .score-box > div {
        border-radius: 0.125rem;
        display: inline-block;
    }

    .gov {
        background-color: rgb(220 252 231)
    }

    .opp {
        background-color: rgb(243 232 255)
    }

    .non-aligned {
        background-color: rgb(254 249 195);
    }
</style>
