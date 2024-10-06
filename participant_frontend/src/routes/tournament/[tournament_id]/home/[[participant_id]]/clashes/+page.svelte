

<script>
    export let data;

    $: clashIds = new Set(data.declared_clashes.map(clash => clash.participant_id));
    $: tabMasterDeclaredClashIds = data.declared_clashes ? new Set(data.declared_clashes.filter(clash => !clash.is_self_declared).map(clash => clash.participant_id)) : new Set();

    $: targets = data.targets ? data.targets : [];
    $: targets.sort((a, b) => {
        let cmp =  -a.participant_role.localeCompare(b.participant_role);
        if (cmp == 0) {
            cmp = a.participant_name.localeCompare(b.participant_name);
        }
        return cmp;
    });

    let filter = "";
</script>

<style>
    table {
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

    .clash_name {
        min-width: 250px;
    }

    table {
        width: 100%;
    }
    .button_cell {
        width: 25px;
        text-align: center;
    }

    h2 {
        font-weight: bold;
        font-size: large;
    }

    .note {
        font-size: small;
        font-style: italic;
    }

    .wrapper {
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: center;
        max-width: 750px;
        margin: auto;
    }
    
    .button {
        padding: 0.5rem;
        border-radius: 0.25rem;
        border: 1px solid #aaa;
        background-color: white;
        cursor: pointer;
        margin-bottom: 1rem;
    }

    input[type="text"] {
        padding: 0.5rem;
        border-radius: 0.25rem;
        border: 1px solid #aaa;
        background-color: white;
        margin-bottom: 1rem;
    }
</style>

<div class="wrapper">
    <h2>Declared Clashes</h2>
    <p class="note">
        This page only shows clashes you have declared, or that the tabmaster entered on your behalf. You can not see clashes other people have declared towards you.
    </p>

    {#if data.isEditing}
    <form method="POST" action="?/updateClashes">
        <button type="submit" class="button">Save</button>
        <input type="text" placeholder="Filter" bind:value={filter} />

        <table>
            <thead>
                <tr>
                    <th class="clash_name">Participant</th>
                    <th class="">Clash?</th>
                </tr>
            </thead>
            <tbody>
                {#each targets as clash}
                    <tr class={
                        !(filter == "" || clash.participant_name.toLowerCase().includes(filter.toLowerCase())) ? "hidden" : ""
                    }>
                        <td>
                            {clash.participant_name}
                            <p class="note">{clash.participant_role == "adjudicator" ? "Adjudicator" : "Speaker"}</p>

                            {#if tabMasterDeclaredClashIds.has(clash.uuid)}
                                <p class="note">Contact the tabmaster if you this clash is wrong.</p>
                            {/if}
                        </td>
                        <td class="button_cell">
                            <input type="checkbox" name="clashes[]" value={clash.uuid} checked={clashIds.has(clash.uuid)} disabled={tabMasterDeclaredClashIds.has(clash.uuid)} />
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>    
        {#each data.declared_clashes as clash}
            <input type="hidden" name="previous_clashes[]" value={clash.participant_id} />
        {/each}
    </form>
    {:else}
    <a class="button" href="?edit">Edit</a>

    <table>
        <thead>
            <tr>
                <th class="clash_name">Clash</th>
            </tr>
        </thead>
        <tbody>
            {#each data.declared_clashes as clash}
                <tr>
                    <td>{clash.participant_name}</td>
                </tr>
            {/each}
        </tbody>
    </table>
    {/if}
</div>