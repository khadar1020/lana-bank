"use client"

import { gql, useApolloClient } from "@apollo/client"
import { useEffect } from "react"

import CreditFacilityDetailsCard from "./details"

import { DetailsPageSkeleton } from "@/components/details-page-skeleton"
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/ui/tab"
import { useTabNavigation } from "@/hooks/use-tab-navigation"

import {
  ApprovalProcessStatus,
  CreditFacilityStatus,
  GetCreditFacilityBasicDetailsDocument,
  GetCreditFacilityDisbursalsDocument,
  GetCreditFacilityOverviewDocument,
  GetCreditFacilityTransactionsDocument,
  useGetCreditFacilityBasicDetailsQuery,
} from "@/lib/graphql/generated"
import { useBreadcrumb } from "@/app/breadcrumb-provider"

gql`
  fragment CreditFacilityBasicDetailsFragment on CreditFacility {
    id
    creditFacilityId
    status
    facilityAmount
    collateralizationState
    customer {
      customerId
      email
    }
    approvalProcess {
      id
      deniedReason
      status
      subjectCanSubmitDecision
      approvalProcessId
      approvalProcessType
      createdAt
    }
    subjectCanUpdateCollateral
    subjectCanInitiateDisbursal
    subjectCanRecordPayment
    subjectCanComplete
  }

  query GetCreditFacilityBasicDetails($id: UUID!) {
    creditFacility(id: $id) {
      ...CreditFacilityBasicDetailsFragment
    }
  }
`

const TABS = [
  { url: "/", tabLabel: "Overview" },
  { url: "/terms", tabLabel: "Terms" },
  { url: "/transactions", tabLabel: "Transactions" },
  { url: "/disbursals", tabLabel: "Disbursals" },
]

export default function CreditFacilityLayout({
  children,
  params,
}: {
  children: React.ReactNode
  params: { "credit-facility-id": string }
}) {
  const { "credit-facility-id": creditFacilityId } = params
  const { currentTab, handleTabChange } = useTabNavigation(TABS, creditFacilityId)
  const { setCustomLinks, resetToDefault } = useBreadcrumb()
  const client = useApolloClient()

  const { data, loading, error, refetch } = useGetCreditFacilityBasicDetailsQuery({
    variables: { id: creditFacilityId },
    fetchPolicy: "cache-and-network",
  })

  useEffect(() => {
    if (
      data?.creditFacility?.status === CreditFacilityStatus.PendingApproval &&
      data?.creditFacility?.approvalProcess?.status === ApprovalProcessStatus.Approved
    ) {
      const timer = setInterval(() => {
        client.refetchQueries({
          include: [
            GetCreditFacilityTransactionsDocument,
            GetCreditFacilityBasicDetailsDocument,
            GetCreditFacilityOverviewDocument,
            GetCreditFacilityDisbursalsDocument,
          ],
        })
      }, 3000)

      return () => clearInterval(timer)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.creditFacility?.status, data?.creditFacility?.approvalProcess?.status])

  useEffect(() => {
    if (data?.creditFacility) {
      const currentTabData = TABS.find((tab) => tab.url === currentTab)
      setCustomLinks([
        { title: "Dashboard", href: "/dashboard" },
        { title: "Credit Facilities", href: "/credit-facilities" },
        {
          title: data.creditFacility.creditFacilityId,
          href: `/credit-facilities/${creditFacilityId}`,
        },
        ...(currentTabData?.url === "/"
          ? []
          : [{ title: currentTabData?.tabLabel ?? "", isCurrentPage: true as const }]),
      ])
    }
    return () => {
      resetToDefault()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.creditFacility, currentTab])

  if (loading) return <DetailsPageSkeleton detailItems={4} tabs={4} />
  if (error) return <div className="text-destructive">{error.message}</div>
  if (!data?.creditFacility) return <div>Not found</div>

  return (
    <main className="max-w-7xl m-auto">
      <CreditFacilityDetailsCard
        creditFacilityId={creditFacilityId}
        creditFacilityDetails={data.creditFacility}
        refetch={refetch}
      />
      <Tabs value={currentTab} onValueChange={handleTabChange} className="mt-2">
        <TabsList>
          {TABS.map((tab) => (
            <TabsTrigger key={tab.url} value={tab.url}>
              {tab.tabLabel}
            </TabsTrigger>
          ))}
        </TabsList>
        {TABS.map((tab) => (
          <TabsContent key={tab.url} value={tab.url}>
            {children}
          </TabsContent>
        ))}
      </Tabs>
    </main>
  )
}