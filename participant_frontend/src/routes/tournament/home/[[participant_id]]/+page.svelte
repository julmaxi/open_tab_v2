<script>
    export let data;

    let currentRounds = data.rounds.filter(round => round.status === 'DrawReleased' || round.status === 'InProgress');
    console.dir(currentRounds, {depth: null});
    let futureRounds = data.rounds.filter(round => round.status === 'Planned');
    let pastRounds = data.rounds.filter(round => round.status === 'Completed')
</script>

<h1>
    Private Page for {data.name}
</h1>

{#each currentRounds as round}
    <div>
        <h2>{round.name}</h2>

        {#if round.participant_role.role === "NotDrawn" }
        <span>You are not in the draw for this round</span>
        {:else if round.participant_role.role === "TeamSpeaker" }
        <span>You are {round.participant_role.team_role} </span>
        {:else if round.participant_role.role === "NonAlignedSpeaker" }
        <span>You are Non Aligned Speaker</span>
        {:else if round.participant_role.role === "Adjudicator" }
        <span>You are {round.participant_role.position == 0 ? "Chair" : "Wing"}</span>
        {:else if round.participant_role.role === "Multiple" }
        <span>There seems to be a mistake in the draw. You have been allocated multiple debates.</span>
        {/if}
        {#if round.participant_role.role !== "Multiple" && round.participant_role.role !== "NotDrawn"}
        {#if round.participant_role.debate.venue !== null}
            in {round.participant_role.debate.venue.name}
        {/if}
        {/if}

        <div>
            {#if round.participant_role.role === "Adjudicator" }
            <a href={`/debate/${round.participant_role.debate.uuid}`}>
                Submit ballot
            </a>
            {/if}
        </div>
    </div>
{/each}

<h2>
    Past Rounds
</h2>

<ul>
{#each pastRounds as round}
    <li>
        <a href="/tournament/[[data.id]]/round/[[round.id]]">
            {round.name}
        </a>
    </li>
{/each}
</ul>

<h2>
    Future Rounds
</h2>

<ul>
{#each futureRounds as round}
    <li>
        <a href="/tournament/[[data.id]]/round/[[round.id]]">
            {round.name}
        </a>
    </li>
{/each}
</ul>

