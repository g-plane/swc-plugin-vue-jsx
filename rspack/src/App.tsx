import { ref } from 'vue';

export default {
  name: 'App',

  setup() {
    const count = ref(0);
    const add = () => count.value++;

    return () => (
      <div class="App">
        <h1>Hello world!</h1>
        <div>
          <a href="https://vuejs.org" target="_blank">11
          </a>
        </div>
        <h1>Rspack + Vue JSX</h1>
        <div class="card">
          <button onClick={add}>count is {count.value}</button>
        </div>
      </div>
    );
  },
};


