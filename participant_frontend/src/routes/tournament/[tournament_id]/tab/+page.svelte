<script>
    import Number from "$lib/Number.svelte";
    import ScoreDetailDisplay from "./ScoreDetailDisplay.svelte";

    export let data;

    let activeTab = "teamTab";
    let teamTab = data.tab.team_tab;
    let speakerTab = data.tab.speaker_tab;
</script>


<div class="container mx-auto my-8">
    <div class="flex">
        <button 
            class="px-4 py-2 border rounded-l focus:outline-none {activeTab === 'teamTab' ? 'bg-blue-500 text-white' : ''}" 
            on:click={() => activeTab = 'teamTab'}>Team Tab</button>
        <button 
            class="px-4 py-2 border-t border-b border-r rounded-r focus:outline-none {activeTab === 'speakerTab' ? 'bg-blue-500 text-white' : ''}" 
            on:click={() => activeTab = 'speakerTab'}>Speaker Tab</button>
    </div>

    {#if activeTab === 'teamTab'}
        <table class="min-w-full mt-4 border-collapse">
            <thead>
                <tr>
                    <th class="w-12 px-4 py-2 border">#</th>
                    <th class="px-4 py-2 border">Team Name</th>
                    <th class="px-4 py-2 border">Total Points</th>
                </tr>
            </thead>
            <tbody>
                {#each teamTab as team}
                    <tr>
                        <td class="w-12 px-4 py-2 border">{team.rank + 1}</td>
                        <td class="px-4 py-2 border">
                            {team.team_name}
                            <ScoreDetailDisplay detailedScores={team.detailed_scores} roundInfo={data.rounds} />
                        </td>
                        <td class="w-12 px-4 py-2 border text-right"><Number number={team.total_points} /></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {:else}
        <table class="min-w-full mt-4 border-collapse">
            <thead>
                <tr>
                    <th class="w-12 px-4 py-2 border">#</th>
                    <th class="px-4 py-2 border">Speaker Name</th>
                    <th class="w-12 px-4 py-2 border">Total Points</th>
                </tr>
            </thead>
            <tbody>
                {#each speakerTab as speaker}
                    <tr>
                        <td class="w-12 px-4 py-2 border">
                            {speaker.rank + 1}
                            <ScoreDetailDisplay detailedScores={speaker.detailed_scores} roundInfo={data.rounds} />
                        </td>
                        <td class="px-4 py-2 border">{speaker.speaker_name}</td>
                        <td class="w-12 px-4 py-2 border text-right"><Number number={speaker.total_points} /></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>