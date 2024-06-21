<script>
    import Card from "$lib/Card.svelte";

    export let data;
    console.log(data);
</script>

<style>
    .card-content {
        padding: 0.5rem;
    }

    .container {
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        justify-content: center;

        max-width: 750px;
        margin: 0 auto;
    }
    
    .item {
        width: 100%;
    }

    .team-container {
        display: flex;
        flex-direction: row;
        justify-content: space-between;
    }

    .team-container p {
        flex: 1 1 0px;
    }

    .team-container p:last-child {
        text-align: right;
    }

    .participants-container {
        display: flex;
        flex-direction: row;
        justify-content: space-between;
    }

    .participants-container p {
        flex: 1 1 0px;
        text-align: center;
    }

    .participants-container p:last-child {
        text-align: right;
    }

    .participants-container p:first-child {
        text-align: left;
    }

    p {
        text-overflow: ellipsis;
        text-wrap: nowrap;
    }

    h3 {
        font-size: small;
        font-style: italic;
    }

    h2 {
        font-weight: bold;
    }
</style>

<div class="container">
    {#each data.debates as debate}
        <div class="item">
        <Card>
            <div slot="content" class="card-content">
                <h2>Debate {debate.debate_index + 1} {#if debate.venue} {debate.venue.venue_name} {/if}</h2>
                <div class="team-container">
                    <p>
                        {debate.government.team_name}
                    </p>
                    <p>
                        {debate.opposition.team_name}
                    </p>
                </div>
                <h3>Non Aligned</h3>
                <div class="participants-container">
                    {#each debate.non_aligned_speakers as speaker}
                        <p>
                            {speaker.participant_name}
                        </p>
                    {/each}
                </div>
                <h3>Adjudicators</h3>
                <div class="participants-container">
                    {#each debate.adjudicators as adjudicator}
                        <p>
                            {adjudicator.participant_name}
                        </p>
                    {/each}
                    {#if debate.president}
                        <p>
                            {debate.president.participant_name} <em>(P)</em>
                        </p>
                    {/if}
                </div>
            </div>
        </Card>
        </div>
    {/each}
</div>