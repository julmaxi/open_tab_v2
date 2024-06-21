<script>
    export let data;
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

        background-color: white;
    }

    th {
        padding: 0.25rem;
        border: 1px solid #ccc;
        background-color: #eee;
    }

    .container {
        margin: auto;
        max-width: 750px;
    }

    h2 {
        font-weight: bold;
        font-size: large;
        margin-top: 4px;
    }

    .anonymous {
        font-style: italic;
    }
</style>

<div class="container">
    <h2>Teams</h2>
    <table>
        <thead>
            <tr>
                <th>Team</th>
                <th>Participant</th>
                <th>Institutions</th>
            </tr>
        </thead>
        <tbody>
            {#each data.teams as team}
                {#each team.members as member}
                    <tr>
                        <td>{team.name}</td>
                        <td class={member.is_anonymous ? "anonymous" : ""}>{member.display_name}</td>
                        <td>
                            {#each member.institutions as institution_id, index}
                                {data.institutions[institution_id].name}{#if index != member.institutions.length - 1},{/if}
                            {/each}
                        </td>
                    </tr>
                {/each}
            {/each}
        </tbody>
    </table>

    <h2>Adjudicators</h2>
    <table>
        <thead>
            <tr>
                <th>Participant</th>
                <th>Institutions</th>
            </tr>
        </thead>
        <tbody>
            {#each data.adjudicators as adjudicator}
                <tr>
                    <td class={adjudicator.is_anonymous ? "anonymous" : ""}>{adjudicator.display_name}</td>
                    <td>
                        {#each adjudicator.institutions as institution_id, index}
                            {data.institutions[institution_id].name}{#if index != adjudicator.institutions.length - 1},{/if}
                        {/each}
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>
</div>