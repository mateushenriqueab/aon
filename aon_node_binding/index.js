const binding = require("./build/Release/aon_binding");

module.exports = {
  jsonToAon(json, rootName) {
    return binding.jsonToAon(json, rootName);
  },

  aonToJson(aon) {
    return binding.aonToJson(aon);
  },

  lastError() {
    return binding.lastError();
  }
};
