import * as ser from "@veil/proto";

export class Server {
  private readonly server: ser.Server;

  constructor() {
    this.server = new ser.Server();
  }
}