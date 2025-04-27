<script>
    import Card from "$lib/Card.svelte";
    import CardButton from "$lib/CardButton.svelte";

    export let data;
    
    let rounds = data.rounds;

    const roundStates = {
        "InProgress": "In Progress",
        "Concluded": "Concluded",
    };

    const roundStateColors = {
        "InProgress": "rgb(74 222 128)",
        "Concluded": "rgb(74 74 74)",
    };
</script>

<style>
    .container {
        margin: auto;
        max-width: 750px;
    }
    .status {
        font-size: small;
        font-style: italic;
    }

    .round_entry {
        border-bottom: 1px solid #ccc;
        padding-left: 0.25rem;
    }

    .round_entry:last-child {
        border-bottom: none;
    }

    .round-list {
        list-style-type: none;
        padding: 0;
        border-radius: 0.25rem;
        background-color: white;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
    }

    h2 {
        font-weight: bold;
        font-size: large;
    }

    h3 {
        font-weight: bold;
    }

    .info-slide {
        font-style: italic;
        font-size: small;
    }

    .card-content {
        padding: 0.25rem;
    }

    .item {
        margin-bottom: 0.5rem;
    }

    .award-recipient-entry {
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
        border-bottom: 1px solid #ccc;
    }

    .award-recipient-entry:first-of-type {
        padding-top: 0;
    }

    .award-recipient-entry:last-child {
        border-bottom: none;
    }

    .award-recipient-header {
        font-weight: bold;
    }

    .award-recipient-subtext {
        font-size: small;
        color: #999
    }
</style>


<div class="container">
    {#if data.awards.length > 0}
    <div>
        <h3>Breaks and Awards</h3>
        
        <div>
            {#each data.awards as award}
                <div class="item">
                    <Card cardType="info">
                        <div slot="content" class="card-content">
                            <h2>{award.name}</h2>

                            {#each award.recipients as recipient}
                                <div class="award-recipient-entry">
                                    {#if recipient.type == "Team"}
                                        <h4 class="award-recipient-header">{recipient.team_name}</h4>
                                        <p class="award-recipient-subtext">
                                            {#each recipient.members as member, index}
                                                {#if index > 0}, {/if}
                                                {member.name}
                                            {/each}
                                        </p>
                                    {:else if recipient.type == "Speaker"}
                                        <h4 class="award-recipient-header">{recipient.name}</h4>
                                        <p class="award-recipient-subtext">
                                            {recipient.team_name}
                                        </p> 
                                    {:else if recipient.type == "Adjudicator"}
                                        <h4 class="award-recipient-header">{recipient.name}</h4>
                                        <p class="award-recipient-subtext">
                                           Adjudicator
                                        </p>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </Card>
                </div>
            {/each}
        </div>

    </div>
    {/if}
    {#if data.rounds.length > 0}
    <div>
        <h3>Rounds</h3>

        <div>
            {#each rounds.toReversed() as round }
                <div class="item">
                    <Card cardType={round.state == "InProgress" ? "info" : "disabled" }>
                        <div slot="content" class="card-content">
                            <h2>{round.round_name}</h2>
                            <div class="status" style="color: {roundStateColors[round.state]}">{roundStates[round.state]}</div>
                            {#if round.motion}
                                <p>{round.motion}</p>
                            {/if}

                            {#if round.info_slide}
                                <p class="info-slide">
                                    {round.info_slide}
                                </p>
                            {/if}
                        </div>
                        <div slot="footer">
                            {#if data.show_draws}
                                <CardButton href={`/tournament/${data.tournamentId}/round/${round.uuid}/draw`} label="View Draw" />
                            {/if}
                        </div>
                    </Card>
                </div>
            {/each}
        </div>
    </div>
    {/if}
</div>