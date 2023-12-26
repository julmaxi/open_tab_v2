<script>
    import ScoreDetailDisplay from "../../tab/ScoreDetailDisplay.svelte";
import BoxButton from "./BoxButton.svelte";
    import ScoreDisplay from "./ScoreDisplay.svelte";
    import { enhance } from "$app/forms";
    import { invalidate } from '$app/navigation';
    import BellAnimation from "$lib/BellAnimation.svelte";

    export let data;

    let currentRounds = data.rounds.filter(round => round.status === 'DrawReleased' || round.status == "WaitingToStart" || round.status === 'InProgress');
    let futureRounds = data.rounds.filter(round => round.status === 'Planned');
    let pastRounds = data.rounds.filter(round => round.status === 'Completed')

    let submittedFeedback = data.feedback_submissions.filter(feedback => feedback.submitted_responses.length > 0);
    let unsubmittedFeedback = data.feedback_submissions.filter(feedback => feedback.submitted_responses.length === 0);
    let unsubmittedFeedbackForCurrentRounds = {};
    let overdueFeedback = []

    for (let round of currentRounds) {
        unsubmittedFeedbackForCurrentRounds[round.uuid] = unsubmittedFeedback.filter(feedback => feedback.debate_id === round?.participant_role?.debate?.uuid);
    }
    let currentDebateIds = currentRounds.map(round => round?.participant_role?.debate?.uuid);

    overdueFeedback = unsubmittedFeedback.filter(feedback => !currentDebateIds.includes(feedback.debate_id));

    function formatDate(date) {
        let hours = date.getHours().toString();
        let minutes = date.getMinutes().toString();

        return `${hours.padStart(2, '0')}:${minutes.padStart(2, '0')}`;
    }
</script>

<style>
    .box {
        border-radius: 0.25rem;
        margin: 0.25rem;
        background-color: white;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
    }

    .box button {
        width: 100%;
        display: flex;
        flex-direction: row;
        padding-top: 7px;
        padding-bottom: 7px;
        padding-left: 8px;
        border-top: 1px solid #ccc;
        font-weight: bold;
        align-items: center;
        line-height: 1rem;
    }

    .box button:disabled {
        background-color: #eee;
    }

    .wrapper {
        width: 100%;
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

    .bell-container {
        width: .75rem;
    }
</style>

<div class="wrapper">
<h1>
    Private Page for {data.name}
</h1>


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
            <span>You are Non Aligned Speaker #{round.participant_role.position + 1}</span>
            {:else if round.participant_role.role === "Adjudicator" }
            <span>You are {round.participant_role.position == 0 ? "Chair" : "Wing"}</span>
            {:else if round.participant_role.role === "President" }
            <span>You are President</span>
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
                    Debate starts at {formatDate(new Date(round.debate_start_time + "+00:00"))}
                </div>
            {/if}

        </div>
        </div>
        <div>
            {#if round.participant_role.role === "Adjudicator" || round.participant_role.role === "President" }
                {#if round.status === 'InProgress' }
                    <form action="?/releaseMotion" method="POST" use:enhance={
                        async () => {
                            return async ({ update, result }) => {
                                round.isReleasingMotion = true;

                                await update();

                                round.participant_role.debate.is_motion_released_to_non_aligned = result.data.isMotionReleasedToNonAligned;
                                round.isReleasingMotion = false;
                            };
                        }
                    }>
                        <input type="hidden" name="release" value={round.participant_role.debate.is_motion_released_to_non_aligned ? "false" : "true"} />
                        <input type="hidden" name="debateId" value={round.participant_role.debate.uuid} />
                        <button type="submit" disabled={round.isReleasingMotion} >
                            {#if round.participant_role.debate.is_motion_released_to_non_aligned}Undo r{:else}R{/if}elease motion to non-aligned
                            <div class="bell-container">
                                {#if round.isReleasingMotion}
                                    <BellAnimation color="black" />
                                {/if}
                            </div>
                        </button>
                    </form>
                {/if}

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

{#if data.role.type == "Speaker" }
<table>
    <thead>
        <tr>
            <th>Round</th>
            <th>Score</th>
            <th>Team Score</th>
            <th>Motion</th>
        </tr>
    </thead>
    <tbody>
        {#each pastRounds as round}
            <tr>
                <td>{round.name}</td>
                <td><ScoreDisplay info={round?.participant_role?.speaker_score} /></td>
                <td>
                    <ScoreDisplay info={round?.participant_role?.team_score} />
                </td>
                <td>{round.motion.motion}</td>
            </tr>
        {/each}
    </tbody> 
</table>
{/if}


{#if data.role.type == "Adjudicator" }
<table>
    <thead>
        <tr>
            <th>Round</th>
            <th>Motion</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {#each pastRounds as round}
            <tr>
                <td>{round.name}</td>
                <td>{round.motion.motion}</td>
                <td>
                    <a class="action" href={`/tournament/${data.tournamentId}/debate/${round.participant_role.debate.uuid}`}>
                        View/Edit Ballot
                    </a>
                </td>
            </tr>
        {/each}
    </tbody> 
</table>
{/if}
</div>