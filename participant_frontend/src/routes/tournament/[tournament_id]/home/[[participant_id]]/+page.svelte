<script>
    import BoxButton from "./BoxButton.svelte";

    export let data;

    let currentRounds = data.rounds.filter(round => round.status === 'DrawReleased' || round.status === 'InProgress');
    console.dir(currentRounds, {depth: null});
    let futureRounds = data.rounds.filter(round => round.status === 'Planned');
    let pastRounds = data.rounds.filter(round => round.status === 'Completed')

    let submittedFeedback = data.feedback_submissions.filter(feedback => feedback.submitted_responses.length > 0);
    let unsubmittedFeedback = data.feedback_submissions.filter(feedback => feedback.submitted_responses.length === 0);
    let unsubmittedFeedbackForCurrentRounds = {};
    let overdueFeedback = []

    for (let round of currentRounds) {
        unsubmittedFeedbackForCurrentRounds[round.uuid] = unsubmittedFeedback.filter(feedback => feedback.round_id === round?.participant_role?.debate?.uuid);
    }
    let currentDebateIds = currentRounds.map(round => round?.participant_role?.debate?.uuid);

    overdueFeedback = unsubmittedFeedback.filter(feedback => !currentDebateIds.includes(feedback.round_id));


</script>

<style>
    .box {
        border-radius: 0.25rem;
        margin: 0.25rem;
        background-color: white;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
    }

    .wrapper {
        background-color: rgb(251, 250, 254);
        width: 100%;
        min-height: 100vh;
        overflow: auto;
        padding: 1rem;
    }

    h1 {
        font-size: 1.25rem;
    }

    h2 {
        font-weight: bold;
    }

    h3 {
        font-weight: bold;
    }

    .box-content {
        padding: 0.25rem;
    }

    .error-box {
        background: linear-gradient(0deg, rgba(184,6,6,1) 0%, rgba(177,0,0,1) 100%);
        color: white;
    }

    table {
        width: 100%;
        border-collapse: collapse;
        border: 1px solid #ccc;
    }

    td {
        padding: 0.25rem;
        border: 1px solid #ccc;
    }

    th {
        padding: 0.25rem;
        border: 1px solid #ccc;
        background-color: #eee;
    }

    .action {
        display: block;
        border-radius: 0.25rem;
        padding: 0.25rem;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
        background-color: #eee;
        text-align: center;
    }
</style>

<div class="wrapper">
<h1>
    Private Page for {data.name}
</h1>


{#each currentRounds as round}
    <div class="box">
        <h2>{round.name}</h2>

        <div class="box-content">
        <div>
            {#if round.participant_role.role === "NotDrawn" }
            <span>You are not in the draw for this round</span>
            {:else if round.participant_role.role === "TeamSpeaker" }
            <span>You are {round.participant_role.team_role} </span>
            {:else if round.participant_role.role === "NonAlignedSpeaker" }
            <span>You are Non Aligned Speaker #{round.participant_role.position}</span>
            {:else if round.participant_role.role === "Adjudicator" }
            <span>You are {round.participant_role.position == 0 ? "Chair" : "Wing"}</span>
            {:else if round.participant_role.role === "Multiple" }
            <span>There seems to be a mistake in the draw. You have been allocated multiple debates.</span>
            {/if}
            {#if round.participant_role.role !== "Multiple" && round.participant_role.role !== "NotDrawn"}
            {#if round.participant_role.debate.venue !== null}
                in {round.participant_role.debate.venue.name} (Room {round.participant_role.debate.debate_index + 1})
            {:else}
                in Room {round.participant_role.debate.debate_index + 1}
            {/if}
            {/if}
        </div>

        <div>
            <h3>
                Motion
            </h3>

            {#if round.motion.type == "Shown"}

            <span>{round.motion.motion}</span>

            {#if round.motion.info_slide !== null}
                <div>
                    <h3>
                        Info Slide
                    </h3>
                    <div>
                        {round.motion.info_slide}
                    </div>
                </div>
            {/if}

            {:else}

            <em>The motion for this round has not yet been released</em>

            {/if}

            {#if round.debate_start_time != null}
                <div>
                    Debate starts at {round.debate_start_time}
                </div>
            {/if}

        </div>
        </div>
        <div>
            {#if round.participant_role.role === "Adjudicator" }
            <BoxButton
                href={`/tournament/${data.tournamentId}/debate/${round.participant_role.debate.uuid}`}
                label="Submit ballot" />
            {/if}
            {#each unsubmittedFeedbackForCurrentRounds[round.uuid] as missingFeedback}
                <BoxButton
                    href={`/tournament/${data.tournamentId}/feedback/${missingFeedback.source_role.type.toLowerCase()}/${missingFeedback.target_role.type.toLowerCase()}/debate/${missingFeedback.debate_id}/for/${missingFeedback.target_id}/from/${missingFeedback.source_id.uuid}`}
                    label={`Submit feedback for ${missingFeedback.target_name}`} />
            {/each}
        </div>
    </div>
{/each}


{#if overdueFeedback.length > 0}
<div class="box error-box">
    <div class="box-content">
        <h2>
            Overdue Feedback
        </h2>
    </div>

    {#each overdueFeedback as feedback}
        <BoxButton href={
            `/tournament/${data.tournamentId}/feedback/${feedback.source_role.type.toLowerCase()}/${feedback.target_role.type.toLowerCase()}/debate/${feedback.debate_id}/for/${feedback.target_id}/from/${feedback.source_id.uuid}`
        } label={`${feedback.target_name} (${feedback.target_role.type}) in ${feedback.round_name}`} />
    {/each}
</div>
{/if}

<h2>
    Submitted Feedback
</h2>

<table>
    <thead>
        <tr>
            <th>For</th>
            <th>Round</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {#each submittedFeedback as feedback}
            <tr>
                <td>{feedback.target_name} ({feedback.target_role.type})</td>
                <td>{feedback.round_name}</td>
                <td>
                    <a class="action" href={`/tournament/${data.tournamentId}/feedback/${feedback.source_role.type.toLowerCase()}/${feedback.target_role.type.toLowerCase()}/debate/${feedback.debate_id}/for/${feedback.target_id}/from/${feedback.source_id.uuid}`}>
                        Update
                    </a>
                </td>
            </tr>
        {/each}
    </tbody>
</table>

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
</div>
