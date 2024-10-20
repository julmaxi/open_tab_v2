

<script>
    export let data;

    $: clashIds = new Set(data.declared_clashes.map(clash => clash.participant_id));
    $: institutionIds = new Set(data.declared_institutions.map(clash => clash.institution_id));
    $: tabMasterDeclaredClashIds = data.declared_clashes ? new Set(data.declared_clashes.filter(clash => !clash.is_self_declared).map(clash => clash.participant_id)) : new Set();

    $: targets = data.targets ? data.targets : [];
    $: targets.sort((a, b) => {
        let cmp =  -(a.participant_role || "").localeCompare((b.participant_role || ""));
        if (cmp == 0) {
            cmp = a.name.localeCompare(b.name);
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

    {#if data.isEditing}
    <form method="POST" action="?/updateClashes">
        <button type="submit" class="button">Save</button>
        <input type="text" placeholder="Filter" bind:value={filter} />

        <input type="hidden" name="clash_category" value={data.isEditing} />

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
                        !(filter == "" || clash.name.toLowerCase().includes(filter.toLowerCase())) ? "hidden" : ""
                    }>
                        <td>
                            {clash.name}
                            {#if clash.participant_role}
                                <p class="note">{clash.participant_role == "adjudicator" ? "Adjudicator" : "Speaker"}</p>
                            {/if}

                            {#if tabMasterDeclaredClashIds.has(clash.uuid)}
                                <p class="note">Contact the tabmaster if you this clash is wrong.</p>
                            {/if}
                        </td>
                        <td class="button_cell">
                            <input type="checkbox" name="clashes[]" value={clash.uuid} checked={
                                (data.isEditing === "clashes" ? clashIds : institutionIds).has(clash.uuid)
                            } />
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>    
        {#each (data.isEditing === "clashes" ? data.declared_clashes : data.declared_institutions) as clash}
            <input type="hidden" name="previous_clashes[]" value={data.isEditing === "clashes" ? clash.participant_id : clash.institution_id} />
        {/each}
    </form>
    {:else}
    <h2>Declared Institutions</h2>

    <a class="button" href="?editInstitutions">Edit</a>

    <table>
        <thead>
            <tr>
                <th class="clash_name">Institution</th>
            </tr>
        </thead>
        <tbody>
            {#each data.declared_institutions as clash}
                <tr>
                    <td>{clash.institution_name}</td>
                </tr>
            {/each}
        </tbody>
    </table>

    <h2>Declared Clashes</h2>
    <p class="note">
        This list only shows clashes you have declared, or that the tabmaster entered on your behalf. You can not see clashes other people have declared towards you.
    </p>

    <a class="button" href="?editClashes">Edit</a>

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