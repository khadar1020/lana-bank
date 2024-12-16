import { print } from "@apollo/client/utilities"
import {
  GetOnBalanceSheetTrialBalanceDocument,
  GetOnBalanceSheetTrialBalanceQuery,
} from "../../lib/graphql/generated"

describe("Trial Balance", () => {
  beforeEach(() => {
    cy.visit("/trial-balance")
  })

  it("should render trial balance with accounts and their balances", () => {
    const currentDate = new Date()
    const lastMonthDate = new Date()
    lastMonthDate.setMonth(lastMonthDate.getMonth() - 1)

    cy.graphqlRequest<{ data: GetOnBalanceSheetTrialBalanceQuery }>(
      print(GetOnBalanceSheetTrialBalanceDocument),
      {
        from: lastMonthDate.toISOString(),
        until: currentDate.toISOString(),
      },
    ).then((response) => {
      response.data.trialBalance?.subAccounts.forEach((account) => {
        cy.contains(account.name).should("exist")
        cy.contains(account.name)
          .closest("tr")
          .within(() => {
            cy.get("td").should("have.length", 4)
          })
      })
    })
  })

  it("should switch between currency types", () => {
    cy.contains("USD").should("exist")
    cy.contains("BTC").should("exist")

    cy.contains("USD").click()
    cy.contains("BTC").click()
  })

  it("should switch between balance layers", () => {
    cy.contains("All").should("exist")
    cy.contains("Settled").should("exist")
    cy.contains("Pending").should("exist")

    cy.contains("All").click()
    cy.contains("Settled").click()
    cy.contains("Pending").click()
  })

  it("should display totals row", () => {
    cy.contains("Totals")
      .closest("tr")
      .within(() => {
        cy.get("td").should("have.length", 4)
      })
  })

  it("should show date range selector", () => {
    cy.contains("Date Range:").should("exist")
  })
})
