const nearAPI = require('near-api-js');

class ChainlinkSimpleNode {
  
  constructor() {
    this.isEvil = false;
    // this.requestSpec = 'getLatestTokenPriceByHashOrSymbol';
    this.requestSpec = 'testingFromCommandLine';
    if (process.argv[2] === "evil") {
      this.isEvil = true;
    }
    this.contractName = 'v0.oracle.testnet';
    this.timedGrabber = this.timedGrabber.bind(this);
    this.timedGrabber();

  }

  async grabbit() {
    const provider = new nearAPI.providers.JsonRpcProvider("https://rpc.nearprotocol.com");
    const nonsensicalResult = await provider.query(`call/${this.contractName}/get_all_requests`, 'AQ4'); // Base 58 of '{}'
    const sensicalResult = JSON.parse(nonsensicalResult.result.map((x) => String.fromCharCode(x)).join(''));
    console.log("sensicalResult", sensicalResult);
    const jsonResult = JSON.parse(sensicalResult);
    console.log("jsonResult", jsonResult);
    let matchingRequest = -1;
    // for (let i = 0; i < jsonResult.length; i++) {
    for (const key in jsonResult) {
      // console.log(`Item ${i} is:`, jsonResult[i]);
      console.log(`Item ${key} is:`, jsonResult[key]);
      if (jsonResult[key].request_spec === this.requestSpec) {
        matchingRequest = key;
        break;
      }
    }

    if (matchingRequest === -1) {
      console.log("Couldn't find a matching specification.")
    } else {
      console.log(`Found a spec we can work on, it's key: ${matchingRequest}`);
      // Reserve a spot on the contract
    }

    const randomKey = nearAPI.KeyPair.fromRandom('ed25519').secretKey;
    console.log('randomKey', randomKey);
    const delimiter = 'Ⓝ';
    const apiResponse = '.191';
    const trimmedRandomKey = randomKey.substr(0, apiResponse.length);
    console.log('trimmedRandomKey', trimmedRandomKey);
    let xoredValue = '';
    // max charcode is 65535, don't think that's a problem actually
    for (let i = 0; i < apiResponse.length; i++) {
      console.log("xoring " + apiResponse.charCodeAt(i) + " and " + trimmedRandomKey.charCodeAt(i) + " = '" + String.fromCharCode(apiResponse.charCodeAt(i) ^ trimmedRandomKey.charCodeAt(i)) + "'" + " from " + (apiResponse.charCodeAt(i) ^ trimmedRandomKey.charCodeAt(i)));

      const literalXor = apiResponse.charCodeAt(i) ^ trimmedRandomKey.charCodeAt(i);
      if (literalXor <= 31) {
        xoredValue += String.fromCharCode(literalXor + 31);
      } else if (literalXor >= 127 && literalXor <= 159 ) {
        xoredValue += String.fromCharCode(literalXor + 33);
      } else {
        xoredValue += String.fromCharCode(literalXor);
      }

      // xoredValue += String.fromCharCode(apiResponse.charCodeAt(i) ^ trimmedRandomKey.charCodeAt(i));
    }
    console.log('xoredValue is: \'' + xoredValue + "'");

    

    // let responseToCharCodes = '';
    // for (let i = 0; i < apiResponse.length; i++) {
    //   responseToCharCodes += apiResponse.charCodeAt(i);
    // }
    // console.log('responseToCharCodes', responseToCharCodes);

    // let randomToCharCodes = '';
    // let randomCharIdx = 0;
    // while (randomCharIdx < randomKey.length && randomToCharCodes.length < responseToCharCodes.length) {
    //   console.log('in here');
    //   randomToCharCodes += randomKey.charCodeAt(randomCharIdx);
    //   randomCharIdx++;
    // }
    // console.log('randomToCharCodes', randomToCharCodes);

    /*
    zamples
    responseToCharCodes   46495749
    randomToCharCodes     51112119

    or

    responseToCharCodes   46495749
    randomToCharCodes     5210980100

     */

    // make sure randomToCharCodes is at least as long as responseToCharCodes

    // if (lengthOfResponseCodes > randomKey.length) {
    //   console.warn("The API response is longer than we've expected");
    //   return;
    // }

    // let matchingRandomChars = randomKey.substr(0, lengthOfResponseCodes);
    // for (let i = 0; i < matchingRandomChars.length; i++) {
    //   randomToCharCodes += matchingRandomChars.charCodeAt(i);
    // }
    // const xoredVal = responseToCharCodes ^ randomToCharCodes;
    // console.log('xoredVal', xoredVal);
  }

  async timedGrabber() {
    await this.grabbit();
    console.log("isEvil: ", this.isEvil);
    if (this.timer !== null) {
      this.timer = setTimeout(this.timedGrabber, 1019);
    }
  };
}

new ChainlinkSimpleNode();