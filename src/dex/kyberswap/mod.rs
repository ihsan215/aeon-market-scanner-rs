// use crate::common::ExchangeTrait;
// use crate::create_exchange;
// use async_trait::async_trait;

// const KYBERSWAP_API_BASE: &str = "https://api.kyberswap.com/v1";

// create_exchange!(Kyberswap);

// TODO: Added chain names ->
/*

  private getApiBaseUrl(chainId: ChainId): string {
    const chainNames = {
      [ChainId.ETHEREUM]: 'ethereum',
      [ChainId.BSC]: 'bsc',
      [ChainId.POLYGON]: 'polygon',
      [ChainId.AVALANCHE]: 'avalanche',
      [ChainId.ARBITRUM]: 'arbitrum',
      [ChainId.OPTIMISM]: 'optimism',
      [ChainId.BASE]: 'base',
      [ChainId.PLASMA]: 'plasma',
      [ChainId.UNICHAIN]: 'unichain',
      [ChainId.SONIC]: 'sonic',
      [ChainId.RONIN]: 'ronin',
      [ChainId.HyperEVM]: 'hyprevm',
      [ChainId.LINEA]: 'linea',
      [ChainId.MANTLE]: 'mantle',
    };

    const chainName = chainNames[chainId] || 'ethereum';
    return `https://aggregator-api.kyberswap.com/${chainName}/api/v1`;
  }


  // get route
        // Step 1: Get the best route using a GET request
      const routeParams = {
        tokenIn: fromToken,
        tokenOut: toToken,
        amountIn: amount,
        gasInclude: true,
        saveGas: 0,
        excludedSources: 'bebop,smardex,dodo', // Exclude problematic sources
      };

      const apiConfig = {
        headers: { 'X-Client-Id': 'wc-arbitrage-bot' },
      };
      const routeResponse = await firstValueFrom(
        this.httpService.get(`${baseUrl}/routes`, {
          params: routeParams,
          ...apiConfig,
        }),
      );


*/
