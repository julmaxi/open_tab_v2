<script>
    import Number from "$lib/Number.svelte";
    import ScoreDetailDisplay from "./ScoreDetailDisplay.svelte";

    export let data;

    let activeTab = "teamTab";
    let teamTab = data.tab.team_tab;
    let speakerTab = data.tab.speaker_tab;
</script>


<style>
    table {
        width: 100%;
        border-collapse: collapse;
        border: 1px solid #ccc;
    }

    td {
        border: 1px solid #ccc;

        padding-left: 0.5rem;
        padding-right: 0.5rem;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
    }

    th {
        padding: 0.25rem;
        border: 1px solid #ccc;
        background-color: #eee;
    }

    .rank {
        text-align: right;
        width: 3rem;
    }

    .score {
        text-align: right;
        width: 3rem;
    }
</style>


<div class="container mx-auto">
    <div class="flex">
        <button 
            class="px-4 py-2 border rounded-l focus:outline-none {activeTab === 'teamTab' ? 'bg-blue-500 text-white' : ''}" 
            on:click={() => activeTab = 'teamTab'}>Team Tab</button>
        <button 
            class="px-4 py-2 border-t border-b border-r rounded-r focus:outline-none {activeTab === 'speakerTab' ? 'bg-blue-500 text-white' : ''}" 
            on:click={() => activeTab = 'speakerTab'}>Speaker Tab</button>
    </div>

    {#if activeTab === 'teamTab'}
        <table>
            <thead>
                <tr>
                    <th>#</th>
                    <th>Team Name</th>
                    <th>Total Points</th>
                </tr>
            </thead>
            <tbody>
                {#each teamTab as team}
                    <tr>
                        <td class="rank">{team.rank + 1}</td>
                        <td>
                            {team.team_name}
                            <ScoreDetailDisplay detailedScores={team.detailed_scores} />
                        </td>
                        <td class="score"><Number number={team.total_score} /></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {:else}
        <table>
            <thead>
                <tr>
                    <th>#</th>
                    <th>Speaker Name</th>
                    <th>Total Points</th>
                </tr>
            </thead>
            <tbody>
                {#each speakerTab as speaker}
                    <tr>
                        <td class="rank">
                            {speaker.rank + 1}
                        </td>
                        <td>
                            {speaker.speaker_name}
                            <ScoreDetailDisplay detailedScores={speaker.detailed_scores} />
                        </td>
                        <td class="score"><Number number={speaker.total_score} /></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>