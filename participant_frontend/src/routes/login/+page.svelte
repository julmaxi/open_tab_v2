
<script>
	import { enhance } from '$app/forms';

    export let data;
    export let form;
</script>

<style>
    ul {
        list-style-type: none;
        padding: 0;
    }

    li {
        color: red;
    }

    form {
        display: flex;
        flex-direction: column;
        align-items: center;
    }

    input {
        margin: 0.5rem;
        padding: 0.5rem;
        border-radius: 0.25rem;
        border: 1px solid #aaa;
        background-color: white;
    }

    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        max-width: 750px;
        margin: auto;
    }

    button {
        padding: 0.5rem;
        border-radius: 0.25rem;
        border: 1px solid #aaa;
        background-color: white;
        cursor: pointer;
    }

    h2 {
        font-weight: bold;
        font-size: large;
    }
</style>

<div class="container">
    <h2>Login</h2>
    <ul>
        {#each form?.errors || [] as error}
            <li>{error}</li>
        {/each}
    </ul>
    <form action="?/create" method="POST" use:enhance>
        <input type="text" placeholder="Email" name="user_name" />
        <input type="password" placeholder="Password" name="password" />
        <button type="submit">Login</button>
    </form>
</div>