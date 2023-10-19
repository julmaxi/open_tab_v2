<script>
    export let data;

    let activeTab = "teamTab";
    console.log(data);
    let teamTab = data.tab.team_tab;
    let speakerTab = data.tab.speaker_tab;

    console.log(teamTab, speakerTab);
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
                        <td class="w-12 px-4 py-2 border">{team.rank}</td>
                        <td class="px-4 py-2 border">{team.team_name}</td>
                        <td class="w-12 px-4 py-2 border text-center">{team.total_points}</td>
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
                        <td class="w-12 px-4 py-2 border">{speaker.rank}</td>
                        <td class="px-4 py-2 border">{speaker.speaker_name}</td>
                        <td class="w-12 px-4 py-2 border text-center">{speaker.total_points}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>