<script>
    export let data;

    let isAnonymous = data.isAnonymous ? "t" : "f";
</script>

<style>
    h1 {
        font-size: 1.25rem;
        font-weight: bold;
    }

    .radio {
        display: flex;
        flex-direction: column;;
        align-items: center;
    }
    
    .wrapper {
        padding: 0.5rem;
        display: flex;
        flex-direction: column;
    }

    button {
        margin-top: 1rem;
        margin-bottom: 1rem;
        padding: 0.5rem;
        border-radius: 0.25rem;
        background-color: rgb(34 197 94);
        color: white;
    }

    form {
        display: flex;
        flex-direction: column;
        align-items: center;
    }
</style>

<div class="wrapper">
    <h1>Settings</h1>
    <form method="POST">
        <div class="radio">
            <div>
                <div>
                    <input id="isAnonymous-0" type="radio" name="isAnonymous" value="f" bind:group={isAnonymous} />
                    <label for="isAnonymous-0">Show my full name on Tab</label>
                </div>
                <div>
                    <input id="isAnonymous-1" type="radio" name="isAnonymous" value="t" bind:group={isAnonymous} />
                    <label for="isAnonymous-1">Show only my initials on Tab</label>
                </div>
            </div>
        </div>

        <button type="submit">Save</button>
    </form>
</div>
