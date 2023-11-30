
<script>
    import { derived, writable } from "svelte/store";
    import { roleToColor, computeScoreTotal } from "$lib/ballot_utils.js";
    import ScoreRow from "$lib/ScoreRow.svelte";

    /** @type {import('./$types').PageData} */
    export let ballot;
    /** @type boolean */
    export let compact = false;

    let rawBallot = ballot;

    let teamMembers = {
        government: rawBallot.government.members,
        opposition: rawBallot.opposition.members,
    };

    let adjudicators = [
        ...rawBallot.adjudicators.map(
            (adjudicator) => {
                return {
                    name: adjudicator.name,
                    uuid: adjudicator.uuid,
                }
            }
        )
    ]

    let speeches = writable(
        rawBallot.speeches.map((speech) => {
            return {
                speaker: speech.speaker,
                scores: rawBallot.adjudicators.map((adjudicator) => {
                    return speech.scores[adjudicator.uuid] || null;
                }),
                role: speech.role,
                position: speech.position,
            };
        })
    );

    let speechTotalScores = derived(speeches, ($speeches) => {
        let totalScores = $speeches.map((speech) => {
            return computeScoreTotal(speech.scores);
        });

        return totalScores;
    });

    let teamTotalSpeechScores = derived(
        [speeches, speechTotalScores],
        ([$speeches, $speechTotalScores]) => {
            let roleScores = $speeches.map((speech, idx) => [
                speech.role,
                $speechTotalScores[idx],
            ]);

            let governmentScores = roleScores
                .filter((roleScore) => roleScore[0] === "government")
                .reduce((total, roleScore) => {
                    return total + roleScore[1];
                }, 0);

            let oppositionScores = roleScores
                .filter((roleScore) => roleScore[0] === "opposition")
                .reduce((total, roleScore) => {
                    return total + roleScore[1];
                }, 0);

            return {
                government: governmentScores,
                opposition: oppositionScores,
            };
        }
    );

    let teamScores = writable({
        government: rawBallot.adjudicators.map((adjudicator) => {
            return rawBallot.government.scores[adjudicator.uuid] || null;
        }),
        opposition: rawBallot.adjudicators.map((adjudicator) => {
            return rawBallot.opposition.scores[adjudicator.uuid] || null;
        }),
    });

    let teamTotalTeamScores = derived(teamScores, ($teamScores) => {
        let totalScores = {
            government: computeScoreTotal($teamScores.government),
            opposition: computeScoreTotal($teamScores.opposition),
        };

        return totalScores;
    });

    let teamTotalScores = derived(
        [teamTotalSpeechScores, teamTotalTeamScores],
        ([$teamTotalSpeechScores, $teamTotalTeamScores]) => {
            return {
                government:
                    $teamTotalSpeechScores.government +
                    $teamTotalTeamScores.government,
                opposition:
                    $teamTotalSpeechScores.opposition +
                    $teamTotalTeamScores.opposition,
            };
        }
    );
</script>

<input type="hidden" name="president" value={rawBallot.president?.uuid || null} />
{#each adjudicators as adjudicator, adjIdx}
    <input type="hidden" name="adjudicators.{adjIdx}" value={adjudicator.uuid} />
{/each}
{#each $speeches as speech, speechIdx}
    <input type="hidden" name="speeches.{speechIdx}.role" value={speech.role} />
    <input type="hidden" name="speeches.{speechIdx}.position" value={speech.position} />
    <ScoreRow
        label={speech.speaker}
        labelOptions={teamMembers[speech.role] || null}
        bind:scores={speech.scores}
        totalScore={$speechTotalScores[speechIdx]}
        color={roleToColor(speech.role)}
        inputPrefix={`speeches.${speechIdx}.`}
        adjudicators={adjudicators}
        on:changeLabel={(event) => {
            let newSpeaker = teamMembers[speech.role].find(
                (member) => member.uuid == event.detail.label
            );

            let newSpeakerPrevSpeech = $speeches.find(
                (prevSpeech) => prevSpeech.speaker?.uuid == newSpeaker.uuid
            );

            if (newSpeakerPrevSpeech) {
                newSpeakerPrevSpeech.speaker = speech.speaker;
            }
            speech.speaker = newSpeaker;

            let emptySpeeches = $speeches.filter(
                (otherSpeech) =>
                    otherSpeech.speaker == null &&
                    otherSpeech.role == speech.role
            );

            if (emptySpeeches.length == 1) {
                let nonAssignedSpeaker = teamMembers[speech.role].filter(
                    (member) => {
                        return (
                            $speeches.find(
                                (prevSpeech) =>
                                    prevSpeech.speaker?.uuid == member.uuid
                            ) == null
                        );
                    }
                )[0];
                emptySpeeches[0].speaker = nonAssignedSpeaker;
            }
        }}
        compact={compact}
    />
{/each}
{#each ["government", "opposition"] as teamRole}
<ScoreRow
    label={rawBallot[teamRole]}
    bind:scores={$teamScores[teamRole]}
    totalScore={$teamTotalTeamScores[teamRole]}
    color={roleToColor(teamRole)}
    inputPrefix={`${teamRole}.`}
    compact={compact}
    maxValue={200}
    adjudicators={adjudicators}
/>
{/each}
<div class="grid grid-cols-2 text-lg text-center">
    {#each ["government", "opposition"] as teamRole}
        <div class="pt-1 last:border-l border-b border-t ml-[-1px] font-bold">
            {$teamTotalScores[teamRole]?.toFixed(2) || "-"}
        </div>
    {/each}
</div>