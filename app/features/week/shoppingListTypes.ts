// Browser-owned presentation pin. Shared canonical contracts remain unchanged.
import type {RecipeRef} from '../../generated/contracts';
export type CataloguePin={revision:string;manifest_digest:string;entries:{recipe:RecipeRef;packet_digest:string}[]};
