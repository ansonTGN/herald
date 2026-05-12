import { createFileRoute } from '@tanstack/react-router'
import { ProductPage } from '@/components/billing/product-page'

export const Route = createFileRoute('/$realmId/manage/products')({
  component: ProductsRoute,
})

function ProductsRoute() {
  const { realmId } = Route.useParams()

  return <ProductPage realmId={realmId} />
}
