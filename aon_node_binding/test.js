const aon = require("./index.js");

const json = `
[
  {
    "id": 1,
    "nome": "Alice",
    "profile": {
      "idade": 30,
      "enderecos": [
        { "cep": "06114020", "rua": "Rua das Dores" },
        { "cep": "06114021", "rua": "Rua Azul" }
      ]
    }
  }
]
`;

console.log("==== JSON → AON ==== ");
const aonText = aon.jsonToAon(json, "users");
console.log(aonText);

console.log("\n==== AON → JSON ==== ");
const back = aon.aonToJson(aonText);
console.log(back);

console.log("\nLastError: ", aon.lastError());
