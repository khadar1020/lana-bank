import type { Meta, StoryObj } from "@storybook/react"
import { MockedProvider } from "@apollo/client/testing"

import CreditFacilityLayout from "../layout"

import CreditFacilityDisbursalsPage from "./page"

import {
  GetCreditFacilityBasicDetailsDocument,
  GetCreditFacilityDisbursalsDocument,
  DisbursalStatus,
} from "@/lib/graphql/generated"
import {
  mockCreditFacility,
  mockCreditFacilityDisbursal,
} from "@/lib/graphql/generated/mocks"

const meta = {
  title: "Pages/CreditFacilities/CreditFacility/Disbursals",
  component: CreditFacilityDisbursalsPage,
  parameters: {
    layout: "fullscreen",
    nextjs: {
      appDirectory: true,
    },
  },
} satisfies Meta<typeof CreditFacilityDisbursalsPage>

export default meta
type Story = StoryObj<typeof meta>

const mockParams = { "credit-facility-id": "test-id" }

const layoutMocks = [
  {
    request: {
      query: GetCreditFacilityBasicDetailsDocument,
      variables: {
        id: "test-id",
      },
    },
    result: {
      data: {
        creditFacility: mockCreditFacility({
          id: "test-id",
        }),
      },
    },
  },
]

const disbursalsMocks = [
  {
    request: {
      query: GetCreditFacilityDisbursalsDocument,
      variables: {
        id: "test-id",
      },
    },
    result: {
      data: {
        creditFacility: mockCreditFacility({
          id: "test-id",
          disbursals: [
            mockCreditFacilityDisbursal({}),
            mockCreditFacilityDisbursal({
              status: DisbursalStatus.Denied,
            }),
          ],
        }),
      },
    },
  },
]

export const Default: Story = {
  args: {
    params: mockParams,
  },
  decorators: [
    (Story) => (
      <MockedProvider mocks={layoutMocks} addTypename={false}>
        <CreditFacilityLayout params={mockParams}>
          <MockedProvider mocks={disbursalsMocks} addTypename={false}>
            <Story />
          </MockedProvider>
        </CreditFacilityLayout>
      </MockedProvider>
    ),
  ],
}